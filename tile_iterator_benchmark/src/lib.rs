use kornia_image::{Image, ImageSize};
use rayon::{
    iter::plumbing::{Producer, ProducerCallback, bridge},
    prelude::*,
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Point2d<T = usize> {
    pub x: T,
    pub y: T,
}

pub(crate) fn find_total_tiles(size: ImageSize, tile_size: usize) -> Point2d {
    Point2d {
        x: size.width.div_ceil(tile_size),
        y: size.height.div_ceil(tile_size),
    }
}

pub(crate) fn find_full_tiles(size: ImageSize, tile_size: usize) -> Point2d {
    Point2d {
        x: size.width / tile_size,
        y: size.height / tile_size,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileInfo<'a, T> {
    pub pos: Point2d,
    pub index: usize,
    pub full_index: usize,
    pub data: &'a [&'a [T]],
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageTile<'a, T> {
    FullTile(TileInfo<'a, T>),
    PartialTile(TileInfo<'a, T>),
}

impl<'a, T> ImageTile<'a, T> {
    pub fn as_slice(&self) -> &'a [&'a [T]] {
        match self {
            ImageTile::FullTile(tile) => tile.data,
            ImageTile::PartialTile(tile) => tile.data,
        }
    }
}

pub struct TileIterator<'a, T> {
    img_data: &'a [T],
    img_size: ImageSize,
    tile_size: usize,
    tiles_dim: Point2d,
    last_tile_px: Point2d,
    next_tile_index: Point2d,
    /// The index of the next tile to be yielded by the iterator (counts all tiles, including partial ones).
    next_index: usize,
    /// The index of the next full (non-partial) tile to be yielded by the iterator.
    next_full_index: usize,
    buffer: Vec<&'a [T]>,
}

impl<'a, T> Clone for TileIterator<'a, T> {
    fn clone(&self) -> Self {
        Self {
            img_data: self.img_data,
            img_size: self.img_size,
            tile_size: self.tile_size,
            tiles_dim: self.tiles_dim,
            last_tile_px: self.last_tile_px,
            next_tile_index: self.next_tile_index,
            next_index: self.next_index,
            next_full_index: self.next_full_index,
            buffer: self.buffer.clone(),
        }
    }
}

impl<'a, T> TileIterator<'a, T> {
    pub fn from_image<const C: usize>(img: &'a Image<T, C>, tile_size: usize) -> Self {
        let img_size = img.size();

        let tiles_len = find_total_tiles(img_size, tile_size);
        let last_tile_px = Point2d {
            x: if img.width() % tile_size == 0 {
                tile_size
            } else {
                img_size.width % tile_size
            },
            y: if img.height() % tile_size == 0 {
                tile_size
            } else {
                img_size.height % tile_size
            },
        };

        Self {
            img_data: img.as_slice(),
            img_size,
            tile_size,
            tiles_dim: tiles_len,
            last_tile_px,
            next_tile_index: Point2d::default(),
            next_index: 0,
            next_full_index: 0,
            buffer: Vec::with_capacity(tile_size),
        }
    }
}

impl<'a, T> Iterator for TileIterator<'a, T> {
    type Item = ImageTile<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Stop iteration if we've processed all tiles
        if self.next_tile_index.y >= self.tiles_dim.y {
            return None;
        }

        // number of horizontal pixels in the current tile
        let tile_x_px = if self.next_tile_index.x == self.tiles_dim.x - 1 {
            self.last_tile_px.x
        } else {
            self.tile_size
        };

        // number of vertical pixels in the current tile
        let tile_y_px = if self.next_tile_index.y == self.tiles_dim.y - 1 {
            self.last_tile_px.y
        } else {
            self.tile_size
        };

        self.buffer.clear();
        for y_px in 0..tile_y_px {
            let row = ((self.next_tile_index.y * self.tile_size) + y_px) * self.img_size.width;
            let start_index = row + (self.next_tile_index.x * self.tile_size);
            let end_index = start_index + tile_x_px;

            let row_pxs = &self.img_data[start_index..end_index];
            self.buffer.push(row_pxs);
        }

        let next_tile_index = self.next_tile_index;
        let index = self.next_index;

        // Update indices
        self.next_tile_index.x += 1;
        if self.next_tile_index.x >= self.tiles_dim.x {
            self.next_tile_index.x = 0;
            self.next_tile_index.y += 1;
        }

        self.next_index += 1;
        let data = unsafe { std::slice::from_raw_parts(self.buffer.as_ptr(), tile_y_px) };

        let tile = if data.len() == self.tile_size && data[0].len() == self.tile_size {
            self.next_full_index += 1;
            ImageTile::FullTile(TileInfo {
                data,
                pos: next_tile_index,
                index,
                full_index: self.next_full_index - 1,
            })
        } else {
            ImageTile::PartialTile(TileInfo {
                data,
                pos: next_tile_index,
                index,
                full_index: self.next_full_index - 1,
            })
        };

        Some(tile)
    }
}

/// NOTE: The Image for TileIterator must have atleast 2 full sized tiles
pub struct ParTileIterator<'a, T> {
    base: TileIterator<'a, T>,
}

impl<'a, T: Sync> IntoParallelIterator for TileIterator<'a, T> {
    type Iter = ParTileIterator<'a, T>;

    type Item = ImageTile<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        ParTileIterator { base: self }
    }
}

impl<'a, T: Sync> ParallelIterator for ParTileIterator<'a, T> {
    type Item = ImageTile<'a, T>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<'a, T: Sync> IndexedParallelIterator for ParTileIterator<'a, T> {
    fn len(&self) -> usize {
        self.base.tiles_dim.x * self.base.tiles_dim.y
    }

    fn drive<C: rayon::iter::plumbing::Consumer<Self::Item>>(self, consumer: C) -> C::Result {
        bridge(self, consumer)
    }

    fn with_producer<CB: ProducerCallback<Self::Item>>(self, callback: CB) -> CB::Output {
        callback.callback(TileIteratorProducer {
            end_index: self.len(),
            full_tiles_dim: find_full_tiles(self.base.img_size, self.base.tile_size),
            base: self.base,
        })
    }
}

pub struct TileIteratorProducer<'a, T> {
    base: TileIterator<'a, T>,
    full_tiles_dim: Point2d,
    end_index: usize,
}

impl<'a, T> Clone for TileIteratorProducer<'a, T> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            end_index: self.end_index,
            full_tiles_dim: self.full_tiles_dim,
        }
    }
}

impl<'a, T: Sync> Producer for TileIteratorProducer<'a, T> {
    type Item = ImageTile<'a, T>;

    type IntoIter = Self;

    fn into_iter(self) -> Self::IntoIter {
        self
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let mut left = self.clone();
        left.end_index = index;

        let right_tile_index = Point2d {
            x: index % self.base.tiles_dim.x,
            y: index / self.base.tiles_dim.x,
        };
        let mut right_tile_full_index = right_tile_index.y * self.full_tiles_dim.x;
        right_tile_full_index += right_tile_index.x.min(self.full_tiles_dim.x - 1);

        let mut right = self;
        right.base.next_index = index;
        right.base.next_tile_index = right_tile_index;
        right.base.next_full_index = right_tile_full_index;

        (left, right)
    }

    fn min_len(&self) -> usize {
        let total_tiles = self.base.tiles_dim.x * self.base.tiles_dim.y;
        let threads = rayon::current_num_threads();
        (total_tiles + threads - 1) / threads
    }
}

impl<'a, T> ExactSizeIterator for TileIteratorProducer<'a, T> {}

impl<'a, T> DoubleEndedIterator for TileIteratorProducer<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

impl<'a, T> Iterator for TileIteratorProducer<'a, T> {
    type Item = ImageTile<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.base.next_index >= self.end_index {
            return None;
        }
        self.base.next()
    }
}
