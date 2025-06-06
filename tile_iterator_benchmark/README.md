# Tile Iterator Benchmark

This project benchmarks different iterator implementations for processing mainly image data in Rust.
The goal is to compare the performance of three iterator types:

1. **Window Iterator**: Iterates over fixed-size windows of the image data. [`slice::windows`](https://doc.rust-lang.org/stable/std/primitive.slice.html#method.windows)
2. **Normal Iterator**: Iterates over each pixel in the image sequentially.
3. **Tile Iterator**: Iterates over tiles of image, where each tile is a square sub-region of the image.

## Project Structure

- **`src/lib.rs`**: Contains the implementation of the `TileIterator` struct and its associated methods.
- **`benches/single_thread.rs`**: Contains the benchmarking code

## How `TileIterator` Works

The `TileIterator` divides an image into smaller rectangular tiles of a fixed size. It allows efficient
processing of image data in chunks, which can be useful for tasks like parallel processing or memory optimization.

## Benchmarking

The benchmarking script downloads three images of varying resolutions:
- **Large Image**: 7042 x 4699 pixels
- **Medium Image**: 1920 x 1281 pixels
- **Small Image**: 640 x 427 pixels

### Benchmark Results

| Iterator Type   | Large Image | Medium Image | Small Image|
|-----------------|-------------|--------------|------------|
| Window Iterator | 26.105 ms   | 1.9250 ms    | 212.29 µs  |
| Normal Iterator | 20.628 ms   | 1.5293 ms    | 169.87 µs  |
| Tile Iterator   | 32.786 ms   | 2.4703 ms    | 275.24 µs  |

### Benchmark Environment

- OS: **_Ubuntu 24.04.2 LTS_**
- Processor: **_AMD Ryzen 7 5800X × 16_**
- Memory: **_32 GiB_**
- rustc: **_1.87.0_**
