use kornia_apriltag::segmentation::GradientInfo;
use kornia_apriltag::union_find::UnionFind;
use kornia_image::{Image, allocator::CpuAllocator};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const MIN_CLUSTER_PIXELS: usize = 5;
const BLACK_COLOR: [u8; 3] = [0, 0, 0];

pub fn debug_connected_components(dst: &mut Image<u8, 3, CpuAllocator>, uf: &mut UnionFind) {
    let dst_slice = dst.as_slice_mut();

    for i in 0..dst_slice.len() / 3 {
        let representative = uf.get_representative(i);
        let i = i * 3;

        if uf.get_set_size(representative) < MIN_CLUSTER_PIXELS {
            dst_slice[i..i + 3].copy_from_slice(&BLACK_COLOR);
            continue;
        }

        // Generate a unique color based on the representative
        let mut hasher = DefaultHasher::new();
        representative.hash(&mut hasher);
        let hash = hasher.finish();
        let color = [
            ((hash >> 16) & 0xFF) as u8,
            ((hash >> 8) & 0xFF) as u8,
            (hash & 0xFF) as u8,
        ];

        dst_slice[i..i + 3].copy_from_slice(&color);
    }
}

pub fn debug_gradient_clusters(
    dst: &mut Image<u8, 3, CpuAllocator>,
    clusters: &HashMap<(usize, usize), Vec<GradientInfo>>,
) {
    let dst_width = dst.width();

    // Make everything black
    for px in dst.as_slice_mut() {
        *px = 0;
    }

    let dst_slice = dst.as_slice_mut();

    for (i, (_, infos)) in clusters.iter().enumerate() {
        let color = [
            (((i + 1) * 37) % 256) as u8,
            (((i + 1) * 59) % 256) as u8,
            (((i + 1) * 83) % 256) as u8,
        ];

        for info in infos {
            let idx = ((info.pos.y / 2) * dst_width + (info.pos.x / 2)) * 3;

            if dst_slice[idx..idx + 3] == [0, 0, 0] {
                dst_slice[idx..idx + 3].copy_from_slice(&color);
            }
        }
    }
}
