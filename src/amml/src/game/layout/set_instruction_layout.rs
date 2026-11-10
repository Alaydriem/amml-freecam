use std::mem::offset_of;

use amml_msvc::Optional;

// the game's CameraInstructionOptions::SetInstruction (5Ch bytes): the preset index, then one optional per option, in
// header order. Every member is trivially copyable, so a whole value can be written over the game's
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SetInstructionLayout {
    pub preset_index: u32,                   // 0x00 mPresetIndex
    pub ease: Optional<[u32; 2]>,            // 0x04 mEase (EasingType, time)
    pub pos: Optional<[f32; 3]>,             // 0x10 mPos (Vec3)
    pub rot: Optional<[f32; 2]>,             // 0x20 mRot (x = pitch, y = yaw)
    pub facing: Optional<[f32; 3]>,          // 0x2C mFacing (Vec3)
    pub view_offset: Optional<[f32; 2]>,     // 0x3C mViewOffset
    pub entity_offset: Optional<[f32; 3]>,   // 0x48 mEntityOffset
    pub default: Optional<bool>,             // 0x58 mDefault
    pub remove_ignore_starting_values: bool, // 0x5A mRemoveIgnoreStartingValuesComponent
}

impl SetInstructionLayout {
    // the set instruction for the preset at `preset_index`: position and rotation (pitch, yaw), every other option off
    pub fn new(preset_index: u32, position: [f32; 3], rotation: [f32; 2]) -> Self {
        Self {
            preset_index,
            ease: Optional::none(),
            pos: Optional::some(position),
            rot: Optional::some(rotation),
            facing: Optional::none(),
            view_offset: Optional::none(),
            entity_offset: Optional::none(),
            default: Optional::none(),
            remove_ignore_starting_values: false,
        }
    }
}

const _: () = assert!(offset_of!(SetInstructionLayout, pos) == 0x10);
const _: () = assert!(offset_of!(SetInstructionLayout, rot) == 0x20);
const _: () = assert!(offset_of!(SetInstructionLayout, default) == 0x58);
const _: () = assert!(size_of::<SetInstructionLayout>() == 0x5C);
