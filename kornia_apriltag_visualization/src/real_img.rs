use kornia_apriltag::{
    decode::{DecodeTagsOpts, GrayModelPair, SharpeningBuffer, decode_tags},
    family::TagFamily,
    quad::{FitQuadOpts, fit_quads},
    segmentation::{find_connected_components, find_gradient_clusters},
    threshold::{TileMinMax, adaptive_threshold},
    union_find::UnionFind,
    utils::Pixel,
};
use kornia_image::{Image, allocator::CpuAllocator};
use kornia_imgproc::color::gray_from_rgb_u8;
use kornia_io::jpeg::read_image_jpeg_rgb8;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 0: Load the Image & Convert it to grayscale
    let img = read_image_jpeg_rgb8("./kornia_apriltag_visualization/data/tags_01.jpg")?;

    let mut grayscale = Image::from_size_val(img.size(), 0u8, CpuAllocator)?;
    gray_from_rgb_u8(&img, &mut grayscale)?;

    // Step 1: Adaptive Threshold
    let mut tile_min_max = TileMinMax::new(grayscale.size(), 4);
    let mut bin = Image::from_size_val(grayscale.size(), Pixel::Skip, CpuAllocator)?;
    adaptive_threshold(&grayscale, &mut bin, &mut tile_min_max, 20)?;

    // Step 2(a): Find Connected Components
    let mut uf = UnionFind::new(bin.as_slice().len());
    find_connected_components(&bin, &mut uf)?;

    // Step 2(b): Find Gradient Clusters
    let mut clusters = HashMap::new();
    find_gradient_clusters(&bin, &mut uf, &mut clusters);

    // Step 3: Find Quads
    let mut quads = fit_quads(
        &bin,
        &TagFamily::TAG36_H11,
        &mut clusters,
        5,
        FitQuadOpts::default(),
    );

    // Step 4: Decode Tags
    let mut gray_model_pair = GrayModelPair::new();
    let mut sharpening_buffer = SharpeningBuffer::new(&TagFamily::TAG36_H11);
    let detection = decode_tags(
        &grayscale,
        &mut quads,
        &DecodeTagsOpts::new(&TagFamily::TAG36_H11, true, 0.25),
        &mut gray_model_pair,
        &mut sharpening_buffer,
    );

    for (i, tag) in detection.iter().enumerate() {
        println!(
            "{}: id: {}, center {:?}, quad: {:#?}, decision_margin: {}",
            i, tag.id, tag.center, tag.quad.corners, tag.decision_margin
        );
    }

    Ok(())
}
