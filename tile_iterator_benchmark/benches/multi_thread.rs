use criterion::{BenchmarkId, Criterion};
use kornia_io::jpeg::read_image_jpeg_rgb8;
use rayon::prelude::*;
use reqwest::blocking::get;
use std::{fs::File, io::copy, path::PathBuf};
use tempfile::{TempDir, tempdir};
use tile_iterator_benchmark::TileIterator;

// 7042 x 4699
const LARGE_PHOTO_URL: &str = "https://images.unsplash.com/photo-1484950763426-56b5bf172dbb?ixlib=rb-4.1.0&q=85&fm=jpg&crop=entropy&cs=srgb";
// 1920 x 1281
const MEDIUM_PHOTO_URL: &str = "https://images.unsplash.com/photo-1484950763426-56b5bf172dbb?ixlib=rb-4.1.0&q=85&fm=jpg&crop=entropy&cs=srgb&w=1920";
// 640 x 427
const SMALL_PHOTO_URL: &str = "https://images.unsplash.com/photo-1484950763426-56b5bf172dbb?ixlib=rb-4.1.0&q=85&fm=jpg&crop=entropy&cs=srgb&w=640";

const TILE_SIZE: usize = 4;

fn download_image<'a>(url: &str, file_name: &str) -> (PathBuf, TempDir) {
    let response = get(url).expect("Failed to download photo");
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let temp_file_path = temp_dir.path().join(file_name);
    let mut temp_file = File::create(&temp_file_path).expect("Failed to create temp file");

    copy(
        &mut response.bytes().expect("Failed to read response").as_ref(),
        &mut temp_file,
    )
    .expect("Failed to write image");

    println!(
        "Image downloaded to {:?}, size: {} bytes",
        temp_file_path,
        temp_file_path.metadata().unwrap().len()
    );
    (temp_file_path, temp_dir)
}

fn benchmark(c: &mut Criterion) {
    let (large_image_path, _large_temp_dir) = download_image(LARGE_PHOTO_URL, "large.jpg");
    let (medium_image_path, _medium_temp_dir) = download_image(MEDIUM_PHOTO_URL, "medium.jpg");
    let (small_image_path, _small_temp_dir) = download_image(SMALL_PHOTO_URL, "small.jpg");

    let large_image = read_image_jpeg_rgb8(large_image_path).unwrap();
    let medium_image = read_image_jpeg_rgb8(medium_image_path).unwrap();
    let small_image = read_image_jpeg_rgb8(small_image_path).unwrap();

    let images = [
        (large_image, "7042 x 4699"),
        (medium_image, "1920 x 1281"),
        (small_image, "640 x 427"),
    ];

    let mut group = c.benchmark_group("IteratorComparisions");

    // Window Iterator
    for img in &images {
        group.bench_with_input(BenchmarkId::new("WindowIterator", img.1), img.1, |b, _| {
            b.iter(|| {
                let window_iter = img.0.as_slice().par_windows(TILE_SIZE * TILE_SIZE);

                window_iter.for_each(|window| {
                    for px in window {
                        std::hint::black_box(px);
                    }
                });
            });
        });
    }

    // Normal Iterator
    for img in &images {
        group.bench_with_input(BenchmarkId::new("NormalIterator", img.1), img.1, |b, _| {
            b.iter(|| {
                img.0.as_slice().par_iter().for_each(|px| {
                    std::hint::black_box(px);
                });
            });
        });
    }

    // // Chunk based Iterator
    for img in &images {
        group.bench_with_input(BenchmarkId::new("ChunckIterator", img.1), img.1, |b, _| {
            b.iter(|| {
                img.0
                    .as_slice()
                    .par_chunks(img.0.width())
                    .for_each(|y_chunk| {
                        for pxs in y_chunk.chunks(TILE_SIZE) {
                            for px in pxs {
                                std::hint::black_box(px);
                            }
                        }
                    });
            });
        });
    }

    // Tile based Iterator
    for img in &images {
        group.bench_with_input(BenchmarkId::new("TileIterator", img.1), img.1, |b, _| {
            b.iter(|| {
                let tile_iter = TileIterator::from_image(&img.0, TILE_SIZE).into_par_iter();

                tile_iter.for_each(|tile| {
                    for px_row in tile.as_slice() {
                        let px_row = *px_row;
                        for px in px_row {
                            std::hint::black_box(px);
                        }
                    }
                });
            })
        });
    }
}

criterion::criterion_group!(benches, benchmark);
criterion::criterion_main!(benches);
