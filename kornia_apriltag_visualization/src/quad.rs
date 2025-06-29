use kornia_apriltag::quad::Quad;
use kornia_image::{
    Image,
    allocator::{CpuAllocator, ImageAllocator},
};
use kornia_imgproc::draw::draw_line;
use std::hash::{DefaultHasher, Hash, Hasher};

pub fn debug_quad_fitting<A: ImageAllocator>(
    src: &Image<u8, 3, A>,
    dst: &mut Image<u8, 3, CpuAllocator>,
    quads: &[Quad],
) {
    dst.as_slice_mut().copy_from_slice(src.as_slice());

    for (i, quad) in quads.iter().enumerate() {
        // Generate Unique Color for each quad
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let hash = hasher.finish();
        let color = [
            ((hash >> 16) & 0xFF) as u8,
            ((hash >> 8) & 0xFF) as u8,
            (hash & 0xFF) as u8,
        ];

        let cords = [
            (quad.corners[0].x as i64, quad.corners[0].y as i64),
            (quad.corners[1].x as i64, quad.corners[1].y as i64),
            (quad.corners[2].x as i64, quad.corners[2].y as i64),
            (quad.corners[3].x as i64, quad.corners[3].y as i64),
        ];

        draw_line(dst, cords[3], cords[2], color, 5); // Top Line
        draw_line(dst, cords[2], cords[1], color, 5); // Right Line
        draw_line(dst, cords[0], cords[1], color, 5); // Bottom Line
        draw_line(dst, cords[3], cords[0], color, 5); // Left Line
    }
}
