use std::mem::offset_of;

// the game's HitResult (88h bytes). The entity reference is padding: a clip without an actor leaves it empty
#[repr(C)]
pub struct HitResultLayout {
    pub start: [f32; 3],      // 0x00 mStartPos
    pub direction: [f32; 3],  // 0x0C mRayDir
    pub kind: i32,            // 0x18 mType (HitResultType)
    pub facing: u8,           // 0x1C mFacing
    pub block: [i32; 3],      // 0x20 mBlock
    pub pos: [f32; 3],        // 0x2C mPos
    _entity: [u64; 3],        // 0x38 mEntity (WeakEntityRef)
    pub entity_box: [f32; 6], // 0x50 mEntityAABB
    pub is_hit_liquid: bool,  // 0x68 mIsHitLiquid
    pub liquid_facing: u8,    // 0x69 mLiquidFacing
    pub liquid: [i32; 3],     // 0x6C mLiquid
    pub liquid_pos: [f32; 3], // 0x78 mLiquidPos
    pub indirect_hit: bool,   // 0x84 mIndirectHit
}

const _: () = assert!(offset_of!(HitResultLayout, kind) == 0x18);
const _: () = assert!(offset_of!(HitResultLayout, pos) == 0x2C);
const _: () = assert!(offset_of!(HitResultLayout, entity_box) == 0x50);
const _: () = assert!(size_of::<HitResultLayout>() == 0x88);
