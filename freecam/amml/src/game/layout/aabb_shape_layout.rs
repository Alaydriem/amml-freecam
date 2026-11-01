// the game's AABBShapeComponent (20h bytes): the box (min x, y, z, max x, y, z), then the width and height
#[repr(C)]
pub struct AabbShapeLayout {
    pub aabb: [f32; 6], // 0x00 mAABB
    pub dim: [f32; 2],  // 0x18 mBBDim
}

const _: () = assert!(size_of::<AabbShapeLayout>() == 0x20);
