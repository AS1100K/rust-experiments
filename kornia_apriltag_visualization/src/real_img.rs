use kornia_apriltag::{AprilTagDecoder, DecodeTagsConfig, family::TagFamilyKind};
use kornia_image::{Image, allocator::CpuAllocator};
use kornia_imgproc::color::gray_from_rgb_u8;
use kornia_io::jpeg::read_image_jpeg_rgb8;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 0: Load the Image & Convert it to grayscale
    let img = read_image_jpeg_rgb8("./kornia_apriltag_visualization/data/tags_01.jpg")?;

    let mut grayscale = Image::from_size_val(img.size(), 0u8, CpuAllocator)?;
    gray_from_rgb_u8(&img, &mut grayscale)?;

    let config = DecodeTagsConfig::new(vec![TagFamilyKind::Tag36H11]);
    let mut decoder = AprilTagDecoder::new(config, grayscale.size())?;

    for _ in 0..100 {
        let detection = decoder.decode(&grayscale)?;
        decoder.clear();

        for (i, tag) in detection.iter().enumerate() {
            println!(
                "{}: id: {}, center {:?}, quad: {:#?}, decision_margin: {}",
                i, tag.id, tag.center, tag.quad.corners, tag.decision_margin
            );
        }
    }

    Ok(())
}
