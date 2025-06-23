use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use kornia_apriltag::{threshold::TileMinMax, union_find::UnionFind, utils::Pixel};
use kornia_image::{Image, ImageSize, allocator::CpuAllocator};
use kornia_io::{fps_counter::FpsCounter, stream::V4L2CameraConfig};

use crate::segmentation::{debug_connected_components, debug_gradient_clusters};

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
        .grab()?
        .ok_or("Failed to fetch the initial frame, to get info about image size")?;

    // Preallocated Image buffers
    let mut grayscale_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;
    let mut binary_image = Image::from_size_val(first_frame.size(), Pixel::Skip, CpuAllocator)?;
    let mut segmentation_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;
    let mut cluster_image = Image::from_size_val(first_frame.size(), 0u8, CpuAllocator)?;

    // Preallocated extras
    let mut tile_min_max = TileMinMax::new(binary_image.size(), 4);
    let mut uf = UnionFind::new(first_frame.width() * first_frame.height());
    let mut clusters = HashMap::new();

    drop(first_frame);

    while !cancel_token.load(Ordering::SeqCst) {
        let Some(img) = webcam.grab()? else {
            continue;
        };

        // Convert to grayscale
        kornia_imgproc::color::gray_from_rgb_u8(&img, &mut grayscale_image)?;

        // Convert to binary
        kornia_apriltag::threshold::adaptive_threshold(
            &grayscale_image,
            &mut binary_image,
            &mut tile_min_max,
            20,
        )?;

        // Find Connected Components
        kornia_apriltag::segmentation::find_connected_components(&binary_image, &mut uf)?;

        // Find Gradient Clusters
        kornia_apriltag::segmentation::find_gradient_clusters(
            &binary_image,
            &mut uf,
            &mut clusters,
        );

        // Log each step in rerun
        rec.log_static(
            "Original Frame",
            &rerun::Image::from_elements(img.as_slice(), img.size().into(), rerun::ColorModel::RGB),
        )?;

        rec.log_static(
            "Grayscale Frame",
            &rerun::Image::from_elements(
                grayscale_image.as_slice(),
                grayscale_image.size().into(),
                rerun::ColorModel::L,
            ),
        )?;

        let binary_slice = unsafe {
            std::slice::from_raw_parts(
                binary_image.as_slice().as_ptr() as *const u8,
                binary_image.as_slice().len(),
            )
        };
        rec.log_static(
            "Adaptive Threshold Frame",
            &rerun::Image::from_elements(binary_slice, img.size().into(), rerun::ColorModel::L),
        )?;

        debug_connected_components(&mut segmentation_image, &mut uf);
        rec.log_static(
            "Connected Components",
            &rerun::Image::from_elements(
                segmentation_image.as_slice(),
                segmentation_image.size().into(),
                rerun::ColorModel::RGB,
            ),
        )?;

        debug_gradient_clusters(&mut cluster_image, &clusters);
        rec.log_static(
            "Gradient Clusters",
            &rerun::Image::from_elements(
                cluster_image.as_slice(),
                cluster_image.size().into(),
                rerun::ColorModel::RGB,
            ),
        )?;

        fps_counter.update();
        uf.reset();
        clusters.clear();
    }

    webcam.close()?;
    println!("Finished recording. Closing app.");

    Ok(())
}
