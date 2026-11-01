use std::mem::offset_of;

use amml_game::HashedStringLayout;

// the game's MinecraftCamera::CameraComponent (120h bytes). `CameraComponent.sizeFactor` and `.sizeShift` check the
// size: the game builds the storage stride from them
#[repr(C)]
pub struct CameraComponentLayout {
    pub id: HashedStringLayout, // 0x000 mId
    pub orientation: [f32; 4],  // 0x030 mOrientation (glm::quat x, y, z, w)
    pub position: [f32; 3],     // 0x040 mPosition
    _projection: [f32; 4],      // 0x04C mAspectRatio, mFieldOfView, mNearPlane, mFarPlane
    _matrices: [[f32; 16]; 3],  // 0x05C mPostViewTransform, mSavedProjection, mSavedModelView
}

const _: () = assert!(offset_of!(CameraComponentLayout, orientation) == 0x30);
const _: () = assert!(offset_of!(CameraComponentLayout, position) == 0x40);
const _: () = assert!(size_of::<CameraComponentLayout>() == 0x120);
