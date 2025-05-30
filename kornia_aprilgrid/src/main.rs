use argh::FromArgs;
use kornia::{image::ImageSize, imgproc::draw::draw_line, io::stream::V4L2CameraConfig};
use rand::{Rng, SeedableRng};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(FromArgs)]
/// Capture frames from a webcam and detect april tag while logging to rerun.
struct Args {
    /// the camera id to use
    #[argh(option, short = 'c', default = "0")]
    camera_id: u32,

    /// the frames per second to record
    #[argh(option, short = 'f', default = "30")]
    fps: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = argh::from_env();
    env_logger::init();
    let rec = rerun::RecordingStreamBuilder::new("Kornia_aprilgrid").connect_grpc()?;

    // Create a wecam object
    let mut webcam = V4L2CameraConfig::new()
        .with_camera_id(args.camera_id)
        .with_fps(args.fps)
        .with_size(ImageSize {
            width: 640,
            height: 480,
        })
        .build()?;

    webcam.start()?;

    // create a cancel token to stop the webcam capture
    let cancel_token = Arc::new(AtomicBool::new(false));

    ctrlc::set_handler({
        let cancel_token = cancel_token.clone();
        move || {
            println!("Received Ctrl-C signal. Sending cancel signal !!");
            cancel_token.store(true, Ordering::SeqCst);
        }
    })?;

    let detector = aprilgrid::detector::TagDetector::new(&aprilgrid::TagFamily::T36H11, None);

    while !cancel_token.load(Ordering::SeqCst) {
        let Some(mut img) = webcam.grab()? else {
            continue;
        };

        let tags = detector.detect_kornia(&img);
        for (tag_id, corners) in tags {
            if corners.len() == 4 {
                // Draw lines connecting the corners to form a quadrilateral
                let line_color = id_to_color(tag_id);
                const LINE_THICKNESS: usize = 2;

                draw_line(
                    &mut img,
                    (corners[0].0 as i64, corners[0].1 as i64),
                    (corners[1].0 as i64, corners[1].1 as i64),
                    line_color,
                    LINE_THICKNESS,
                );
                draw_line(
                    &mut img,
                    (corners[1].0 as i64, corners[1].1 as i64),
                    (corners[2].0 as i64, corners[2].1 as i64),
                    line_color,
                    LINE_THICKNESS,
                );
                draw_line(
                    &mut img,
                    (corners[2].0 as i64, corners[2].1 as i64),
                    (corners[3].0 as i64, corners[3].1 as i64),
                    line_color,
                    LINE_THICKNESS,
                );
                draw_line(
                    &mut img,
                    (corners[3].0 as i64, corners[3].1 as i64),
                    (corners[0].0 as i64, corners[0].1 as i64),
                    line_color,
                    LINE_THICKNESS,
                );
            }
        }

        rec.log_static(
            "live_camera",
            &rerun::Image::from_elements(img.as_slice(), img.size().into(), rerun::ColorModel::RGB),
        )?;
    }

    // NOTE: this is important to close the webcam properly, otherwise the app will hang
    webcam.close()?;
    Ok(())
}

fn id_to_color(id: u32) -> [u8; 3] {
    let mut small_rng = rand::rngs::SmallRng::seed_from_u64(id as u64);
    let color_num = small_rng.random_range(0..2u32.pow(24));

    [
        ((color_num >> 16) % 256) as u8,
        ((color_num >> 8) % 256) as u8,
        (color_num % 256) as u8,
    ]
}
