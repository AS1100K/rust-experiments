use apriltag::DetectorBuilder;
use criterion::{Criterion, criterion_group, criterion_main};
use kornia_apriltag::{
    decode::{DecodeTagsOpts, Detection, GrayModelPair, SharpeningBuffer},
    family::TagFamily,
    quad::FitQuadOpts,
    segmentation::GradientInfo,
    threshold::TileMinMax,
    union_find::UnionFind,
    utils::Pixel,
};
use kornia_image::{Image, ImageSize, allocator::CpuAllocator};
use kornia_imgproc::color::gray_from_rgb_u8;
use kornia_io::png::read_image_png_rgba8;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;

const TAG36H11_DIR: &str = "./apriltag_imgs/tag36h11";
const SCALE_FACTOR: usize = 6;
const IMAGE_SIZE: ImageSize = ImageSize {
    width: 60,
    height: 60,
};
const TILE_SIZE: usize = 4;

fn bench(c: &mut Criterion) {
    let imgs_iter = std::fs::read_dir(TAG36H11_DIR).unwrap();
    let imgs_count = imgs_iter.count();

    let mut apriltag_c_detector = DetectorBuilder::new()
        .add_family_bits(apriltag::Family::tag_36h11(), 2)
        .build()
        .unwrap();

    apriltag_c_detector.set_thread_number(1);
    apriltag_c_detector.set_decimation(1.0);
    apriltag_c_detector.set_refine_edges(true);
    apriltag_c_detector.set_thresholds(apriltag::detector::QuadThresholds {
        min_cluster_pixels: 5,
        max_maxima_number: 10,
        min_angle: measurements::Angle::from_radians(0.0),
        min_opposite_angle: measurements::Angle::from_radians(0.984808),
        max_mse: noisy_float::NoisyFloat::new(10.0),
        min_white_black_diff: 20,
        deglitch: false,
    });
    apriltag_c_detector.set_shapening(0.25);

    let mut tile_min_max = TileMinMax::new(IMAGE_SIZE, TILE_SIZE);
    let mut uf = UnionFind::new(IMAGE_SIZE.width * IMAGE_SIZE.height);
    let mut clusters = HashMap::new();
    let mut gray_model_pair = GrayModelPair::new();
    let mut sharpening_buffer = SharpeningBuffer::new(&TagFamily::TAG36_H11);

    let fit_quad_opts = FitQuadOpts::default();
    let decode_tags_opts = DecodeTagsOpts::new(&TagFamily::TAG36_H11, true, 0.5);

    let mut original_rgb = Image::from_size_val([10, 10].into(), 0u8, CpuAllocator).unwrap();
    let mut threshold = Image::from_size_val(IMAGE_SIZE, Pixel::Skip, CpuAllocator).unwrap();

    let mut apriltag_c_images = Vec::with_capacity(imgs_count); // Rough Estimation of number of images
    let mut kornia_images = Vec::with_capacity(imgs_count);

    // Pre-processing
    let imgs = std::fs::read_dir(TAG36H11_DIR).unwrap();
    for img in imgs {
        let img = img.unwrap();

        let file_name = img.file_name();
        let file_name = file_name.to_str().unwrap();

        if file_name.starts_with("tag36_11_") {
            let file_path = img.path();

            let original_img = read_image_png_rgba8(file_path).unwrap();
            rgb_from_rgba(&original_img, &mut original_rgb);

            let mut grayscale = Image::from_size_val([10, 10].into(), 0u8, CpuAllocator).unwrap();
            let mut grayscale_upscale =
                Image::from_size_val(IMAGE_SIZE, 0u8, CpuAllocator).unwrap();
            gray_from_rgb_u8(&original_rgb, &mut grayscale).unwrap();
            scale_image(&grayscale, &mut grayscale_upscale, SCALE_FACTOR);

            let mut apriltag_c_img = apriltag::Image::zeros_with_stride(
                IMAGE_SIZE.width,
                IMAGE_SIZE.height,
                IMAGE_SIZE.width,
            )
            .unwrap();
            grayscale_upscale
                .as_slice()
                .par_iter()
                .zip(apriltag_c_img.as_slice_mut())
                .for_each(|(src, dst)| {
                    *dst = *src;
                });

            kornia_images.push(grayscale_upscale);
            apriltag_c_images.push(apriltag_c_img);
        }
    }

    c.bench_function("kornia-apriltag", |b| {
        b.iter(|| {
            for src in &kornia_images {
                std::hint::black_box(kornia_detection(
                    src,
                    &mut threshold,
                    &mut tile_min_max,
                    &mut uf,
                    &mut clusters,
                    fit_quad_opts,
                    &decode_tags_opts,
                    &mut gray_model_pair,
                    &mut sharpening_buffer,
                ))
                .unwrap();
            }
        });
    });

    c.bench_function("apriltag-c", |b| {
        b.iter(|| {
            for image in &apriltag_c_images {
                std::hint::black_box(apriltag_c_detector.detect(image));
            }
        })
    });
}

fn kornia_detection(
    src: &Image<u8, 1, CpuAllocator>,
    bin: &mut Image<Pixel, 1, CpuAllocator>,
    tile_min_max: &mut TileMinMax,
    uf: &mut UnionFind,
    clusters: &mut HashMap<(usize, usize), Vec<GradientInfo>>,
    fit_quad_opts: FitQuadOpts,
    decode_tags_opts: &DecodeTagsOpts<'static>,
    gray_model_pair: &mut GrayModelPair,
    sharpening_buffer: &mut SharpeningBuffer,
) -> Result<Vec<Detection<'static>>, Box<dyn std::error::Error>> {
    use kornia_apriltag::decode::decode_tags;
    use kornia_apriltag::quad::fit_quads;
    use kornia_apriltag::segmentation::{find_connected_components, find_gradient_clusters};
    use kornia_apriltag::threshold::adaptive_threshold;

    // Step 1: Adaptive Threshold
    adaptive_threshold(&src, bin, tile_min_max, 5)?;

    // Step 2(a): Find Connected Components
    find_connected_components(bin, uf)?;
    // Step 2(b): Find Clusters
    find_gradient_clusters(bin, uf, clusters);

    // Step 3: Quad Fitting
    let mut quads = fit_quads(bin, &TagFamily::TAG36_H11, clusters, 20, fit_quad_opts);

    // Step 3: Tag Decoding
    let detections = decode_tags(
        src,
        &mut quads,
        decode_tags_opts,
        gray_model_pair,
        sharpening_buffer,
    );

    uf.reset();
    clusters.clear();
    gray_model_pair.reset();
    sharpening_buffer.reset();

    Ok(detections)
}

fn rgb_from_rgba(src: &Image<u8, 4, CpuAllocator>, dst: &mut Image<u8, 3, CpuAllocator>) {
    if src.size() != dst.size() {
        panic!("The src and dst size doesn't match");
    }

    src.as_slice()
        .chunks(4)
        .zip(dst.as_slice_mut().chunks_mut(3))
        .for_each(|(src, dst)| {
            dst.copy_from_slice(&src[0..3]);
        });
}

/// A utility function to scale image
fn scale_image(
    src: &Image<u8, 1, CpuAllocator>,
    dst: &mut Image<u8, 1, CpuAllocator>,
    factor: usize,
) {
    if dst.width() != factor * src.width() || dst.height() != factor * src.height() {
        panic!(
            "Destination Image Size didn't matched. src: {:?}, dst: {:?}",
            src.size(),
            dst.size()
        ); // TODO: Use Results
    }

    let src_slice = src.as_slice();
    // To avoid mutable aliasing issues with parallelism, split dst_slice into non-overlapping chunks per row
    let dst_width = dst.width();
    let src_width = src.width();
    let src_height = src.height();

    let dst_ptr = dst.as_mut_ptr();

    // Parallelize over source rows, but only pass unique mutable slices to each thread
    (0..src_height).for_each(|src_y| {
        for src_x in 0..src_width {
            let src_idx = src_y * src_width + src_x;
            let src_px = src_slice[src_idx];
            for dy in 0..factor {
                let dst_y = src_y * factor + dy;
                let row_offset = dst_y * dst_width;
                for dx in 0..factor {
                    let dst_x = src_x * factor + dx;
                    let dst_idx = row_offset + dst_x;
                    // SAFETY: Each thread writes to a unique region of dst, no aliasing
                    unsafe {
                        *dst_ptr.add(dst_idx) = src_px;
                    }
                }
            }
        }
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
