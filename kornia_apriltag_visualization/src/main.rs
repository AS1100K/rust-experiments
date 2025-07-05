use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use kornia_apriltag::{
    decode::{DecodeTagsOpts, GrayModelPair, SharpeningBuffer},
    family::TagFamily,
    quad::FitQuadOpts,
    threshold::TileMinMax,
    union_find::UnionFind,
    utils::Pixel,
};
use kornia_image::{Image, ImageSize, allocator::CpuAllocator};
use kornia_io::{fps_counter::FpsCounter, stream::V4L2CameraConfig};

use crate::{
    quad::debug_quad_fitting,
    segmentation::{debug_connected_components, debug_gradient_clusters},
};

mod quad;
mod segmentation;

// TODO: add CLI arguments
const CAMERA_ID: u32 = 0;
const FPS: u32 = 30;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rec = rerun::RecordingStreamBuilder::new("Kornia-apriltag visualization").spawn()?;

    // Create and start the webcam pipeline
    let mut webcam = V4L2CameraConfig::new()
        .with_camera_id(CAMERA_ID)
        .with_fps(FPS)
        .with_size(ImageSize {
            width: 640,
            height: 480,
        })
        .build()?;
    webcam.start()?;

    let cancel_token = Arc::new(AtomicBool::new(false));
    let mut fps_counter = FpsCounter::new();

    ctrlc::set_handler({
        let cancel_token = cancel_token.clone();
        move || {
            println!("Received Ctrl-C signal. Sending cancel signal!");
            cancel_token.store(true, Ordering::SeqCst);
        }
    })?;

    println!("Waiting for 500 ms!");
    std::thread::sleep(std::time::Duration::from_millis(500));

    let first_frame = webcam
        .grab_rgb8()?
        .ok_or("Failed to fetch the initial frame, to get info about image size")?;

    // Preallocated Image buffers
    let mut grayscale_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;
    let mut binary_image = Image::from_size_val(first_frame.size(), Pixel::Skip, CpuAllocator)?;
    let mut segmentation_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;
    let mut cluster_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;
    let mut quads_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;
    // let mut detections_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;

    // Preallocated extras
    let mut tile_min_max = TileMinMax::new(binary_image.size(), 4);
    let mut uf = UnionFind::new(first_frame.width() * first_frame.height());
    let mut clusters = HashMap::new();
    let mut gray_model_pair = GrayModelPair::new();
    let mut sharpening_buffer = SharpeningBuffer::new(&TagFamily::TAG36_H11);

    let fit_quad_opts = FitQuadOpts::default();
    let decode_tags_opts = DecodeTagsOpts::new(&TagFamily::TAG36_H11, true, 0.25);

    drop(first_frame);

    while !cancel_token.load(Ordering::SeqCst) {
        let Some(img) = webcam.grab_rgb8()? else {
            continue;
        };

        // TEMP FIX: to avoid crash due to gstreamer
        let img = Image::from_size_slice(img.size(), img.as_slice(), CpuAllocator)?;

        rec.log(
            "Original Frame",
            &rerun::Image::from_elements(img.as_slice(), img.size().into(), rerun::ColorModel::RGB),
        )?;

        rec.log(
            "Detected Tags",
            &rerun::Image::from_elements(img.as_slice(), img.size().into(), rerun::ColorModel::RGB),
        )?;

        // Convert to grayscale
        kornia_imgproc::color::gray_from_rgb_u8(&img, &mut grayscale_image)?;

        // rec.log(
        //     "Grayscale Frame",
        //     &rerun::Image::from_elements(
        //         grayscale_image.as_slice(),
        //         grayscale_image.size().into(),
        //         rerun::ColorModel::L,
        //     ),
        // )?;

        // Convert to binary
        kornia_apriltag::threshold::adaptive_threshold(
            &grayscale_image,
            &mut binary_image,
            &mut tile_min_max,
            20,
        )?;

        let binary_slice = unsafe {
            std::slice::from_raw_parts(
                binary_image.as_slice().as_ptr() as *const u8,
                binary_image.as_slice().len(),
            )
        };
        rec.log(
            "Adaptive Threshold Frame",
            &rerun::Image::from_elements(binary_slice, img.size().into(), rerun::ColorModel::L),
        )?;

        // Find Connected Components
        kornia_apriltag::segmentation::find_connected_components(&binary_image, &mut uf)?;

        debug_connected_components(&mut segmentation_image, &mut uf);
        rec.log(
            "Connected Components",
            &rerun::Image::from_elements(
                segmentation_image.as_slice(),
                segmentation_image.size().into(),
                rerun::ColorModel::RGB,
            ),
        )?;

        // Find Gradient Clusters
        kornia_apriltag::segmentation::find_gradient_clusters(
            &binary_image,
            &mut uf,
            &mut clusters,
        );

        debug_gradient_clusters(&mut cluster_image, &clusters);
        rec.log(
            "Gradient Clusters",
            &rerun::Image::from_elements(
                cluster_image.as_slice(),
                cluster_image.size().into(),
                rerun::ColorModel::RGB,
            ),
        )?;

        // Quad Fitting
        // TODO: Avoid multiple allocations
        let mut quads = kornia_apriltag::quad::fit_quads(
            &binary_image,
            &TagFamily::TAG36_H11,
            &mut clusters,
            5,
            fit_quad_opts,
        );

        debug_quad_fitting(&img, &mut quads_image, &quads);
        rec.log(
            "Quads",
            &rerun::Image::from_elements(
                quads_image.as_slice(),
                quads_image.size().into(),
                rerun::ColorModel::RGB,
            ),
        )?;

        // Detect AprilTag
        let detections = kornia_apriltag::decode::decode_tags(
            &grayscale_image,
            &mut quads,
            &decode_tags_opts,
            &mut gray_model_pair,
            &mut sharpening_buffer,
        );

        // Collect all tag quads and labels to draw all at once
        let mut all_coords = Vec::new();
        let mut all_labels = Vec::new();

        for tag in &detections {
            let coords = [
                [tag.quad.corners[0].x, tag.quad.corners[0].y],
                [tag.quad.corners[1].x, tag.quad.corners[1].y],
                [tag.quad.corners[2].x, tag.quad.corners[2].y],
                [tag.quad.corners[3].x, tag.quad.corners[3].y],
                [tag.quad.corners[0].x, tag.quad.corners[0].y],
            ];
            all_coords.push(coords);
            all_labels.push(tag.id.to_string());
        }

        rec.log(
            "Detected Tags",
            &rerun::LineStrips2D::new(all_coords).with_labels(all_labels),
        )?;

        fps_counter.update();
        uf.reset();
        clusters.clear();
        gray_model_pair.reset();
        sharpening_buffer.reset();
    }

    webcam.close()?;
    println!("Finished recording. Closing app.");

    Ok(())
}
