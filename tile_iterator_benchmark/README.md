# Tile Iterator Benchmark

This project benchmarks different iterator implementations for processing mainly image data in Rust.
The goal is to compare the performance of three iterator types:

1. **Window Iterator**: Iterates over fixed-size windows of the image data. [`slice::windows`](https://doc.rust-lang.org/stable/std/primitive.slice.html#method.windows)
2. **Normal Iterator**: Iterates over each pixel in the image sequentially.
3. **Chunks Iterator**: Similar to Tile Iterator but uses [`slice::chunks`](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks) while iterating over
    `1 x TILE_SIZE`.
4. **Tile Iterator**: Iterates over tiles of image i.e. `TILE_SIZE x TILE_SIZE`, where each tile is a square sub-region of the image.
    They are two implementations of Tile Iterator:
    1. Vec based: Returns `Vec<&'a [T]>`. [See commit `8a6286`](https://github.com/AS1100K/rust-experiments/commit/8a6286c2a2439fbbeee34c5c8629c078b471b510)
    2. Slice based: Returns `&'a [&'a [T]]` and avoid creation of data again and again.

## Project Structure

- **`src/lib.rs`**: Contains the implementation of the `TileIterator` struct and its associated methods.
- **`benches/single_thread.rs`**: Contains the benchmarking code

## How `TileIterator` Works

The `TileIterator` divides an image into smaller rectangular tiles of a fixed size. It allows efficient
processing of image data in chunks, which can be useful for tasks like parallel processing or memory optimization.

### Safety

Using slice based `TileIterator` is not safe as you can't use all the Iterator function otherwise the program will crash. But using it works fine in `for_each` call as the
data returned by the iterator doesn't lives long enough and is only valid until the next `Iterator::next` call. There is a discussion on
[Rust Zulip](https://rust-lang.zulipchat.com/#narrow/channel/122651-general/topic/Safe.20implementation.20of.20TileIterator.20yielding.20borrowed.20slices/near/525086420)
if you are interested.

## Benchmarking

The benchmarking script downloads three images of varying resolutions:
- **Large Image**: 7042 x 4699 pixels
- **Medium Image**: 1920 x 1281 pixels
- **Small Image**: 640 x 427 pixels

### Benchmark Results (AMD Ryzen 7 5800X)

| Iterator Type         | Large Image | Medium Image | Small Image|
|-----------------------|-------------|--------------|------------|
| Window Iterator       | 168.17 ms   | 12.508 ms    | 1.3877 ms  |
| Normal Iterator       | 20.773 ms   | 1.5397 ms    | 170.64 µs  |
| Chunks Iterator       | 30.192 ms   | 2.2656 ms    | 257.30 µs  |
| Tile Iterator (vec)   | 41.775 ms   | 3.1059 ms    | 344.15 µs  |
| Tile Iterator (slice) | 20.770 ms   | 1.5495 ms    | 172.96 µs  |

### Benchmark Environment

- OS: **_Ubuntu 24.04.2 LTS_**
- Processor: **_AMD Ryzen 7 5800X × 16_**
- Memory: **_32 GiB_**
- rustc: **_1.87.0_**

### Benchmark Results (NVIDIA Orin Nano 8GB Developer Kit)

| Iterator Type         | Large Image | Medium Image | Small Image|
|-----------------------|-------------|--------------|------------|
| Window Iterator       | 462.14 ms   | 34.334 ms    | 3.8129 ms  |
| Normal Iterator       | 57.502 ms   | 4.2748 ms    | 475.15 µs  |
| Chunks Iterator       | ---         | ---          | ---        |
| Tile Iterator (vec)   | 90.855 ms   | 6.7772 ms    | 750.29 µs  |
| Tile Iterator (slice) | 44.467 ms   | 3.3949 ms    | 377.95 µs  |

### Benchmark Environment

- OS: **_Ubuntu 24.04.5 LTS_**
- Device: **_NVIDIA Orin Nano 8GB Developer Kit_**
- rustc: **_1.87.0_**
