use kornia_image::{Image, ImageSize};

pub struct TileIterator<'a, T> {
    data: &'a [T],
    img_size: ImageSize,
    tile_size: usize,
    /// number of horizontal tiles
    tiles_x_len: usize,
    /// number of vertical tiles
    tiles_y_len: usize,
    /// horizontal pixels available in last tile
    last_tile_x_px: usize,
    /// vertical pixels available in last tile
    last_tile_y_px: usize,
    next_tile_x_index: usize,
    next_tile_y_index: usize,
}

impl<'a, T> TileIterator<'a, T> {
    pub fn from_image<const C: usize>(img: &'a Image<T, C>, tile_size: usize) -> Self {
        let tiles_x_len = (img.width() as f32 / tile_size as f32).ceil() as usize;
        let tiles_y_len = (img.height() as f32 / tile_size as f32).ceil() as usize;

        let last_tile_x_px = if img.width() % tile_size == 0 {
            tile_size
        } else {
            img.width() % tile_size
        };

        let last_tile_y_px = if img.height() % tile_size == 0 {
            tile_size
        } else {
            img.height() % tile_size
        };

        Self {
            data: img.as_slice(),
            img_size: img.size(),
            tile_size,
            tiles_x_len,
            tiles_y_len,
            last_tile_x_px,
            last_tile_y_px,
            next_tile_x_index: 0,
            next_tile_y_index: 0,
        }
    }
}

impl<'a, T> Iterator for TileIterator<'a, T> {
    type Item = Vec<&'a [T]>;

    fn next(&mut self) -> Option<Self::Item> {
        // Stop iteration if we've processed all tiles
        if self.next_tile_y_index >= self.tiles_y_len {
            return None;
        }

        // number of horizontal pixels in the current tile
        let tile_x_px = if self.next_tile_x_index == self.tiles_x_len - 1 {
            self.last_tile_x_px
        } else {
            self.tile_size
        };

        // number of vertical pixels in the current tile
        let tile_y_px = if self.next_tile_y_index == self.tiles_y_len - 1 {
            self.last_tile_y_px
        } else {
            self.tile_size
        };

        let mut buffer = Vec::with_capacity(self.tile_size);

        for y_px in 0..tile_y_px {
            let row = ((self.next_tile_y_index * self.tile_size) + y_px) * self.img_size.width;
            let start_index = row + (self.next_tile_x_index * self.tile_size);
            let end_index = start_index + tile_x_px;

            let row_pxs = &self.data[start_index..end_index];
            buffer.push(row_pxs);
        }

        // Update indices
        self.next_tile_x_index += 1;
        if self.next_tile_x_index >= self.tiles_x_len {
            self.next_tile_x_index = 0;
            self.next_tile_y_index += 1;
        }

        Some(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kornia_image::Image;

    #[test]
    fn test_tile_iterator() {
        let data = vec![127u8; 100];
        let image: Image<_, 1> = Image::new(
            ImageSize {
                width: 25,
                height: 4,
            },
            data,
        )
        .unwrap();

        let tile_iter = TileIterator::from_image(&image, 4);
        let mut counter = 0;

        for tile in tile_iter {
            for tile_row in tile {
                for px in tile_row {
                    assert_eq!(*px, 127);
                    counter += 1;
                }
            }
        }

        assert_eq!(counter, 100);
    }
}
