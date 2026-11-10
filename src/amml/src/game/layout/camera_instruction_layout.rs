use std::mem::offset_of;

use amml_msvc::Optional;

use crate::game::layout::SetInstructionLayout;

// the game's CameraInstruction (150h bytes). The members that hold strings or vectors (attach, spline, target) are
// padding: freecam never writes them, and they stay empty from createPacket. The `CameraInstruction.mSpline` entry
// checks the front (the length the game copies for the trivially copyable part) and `CameraInstructionPacket.size`
// the whole
#[repr(C)]
pub struct CameraInstructionLayout {
    _attach_to_entity: [u64; 3],             // 0x000 mAttachToEntity
    _fade: [u32; 9],                         // 0x018 mFade
    _field_of_view: [u32; 5],                // 0x03C mFieldOfView
    pub set: Optional<SetInstructionLayout>, // 0x050 mSet
    _spline: [u64; 15],                      // 0x0B0 mSpline
    _target: [u64; 4],                       // 0x128 mTarget
    pub clear: Optional<bool>,               // 0x148 mClear
    _detach_from_entity: Optional<bool>,     // 0x14A mDetachFromEntity
    _remove_target: Optional<bool>,          // 0x14C mRemoveTarget
}

impl CameraInstructionLayout {
    // the offset of mSpline, the value the `CameraInstruction.mSpline` entry must have
    pub const SPLINE: u64 = offset_of!(CameraInstructionLayout, _spline) as u64;
    // the bytes the packet holds after the instruction: mSerializationMode, then the padding to 8
    pub const PACKET_TAIL: u64 = 8;
}

const _: () = assert!(offset_of!(CameraInstructionLayout, set) == 0x50);
const _: () = assert!(offset_of!(CameraInstructionLayout, _spline) == 0xB0);
const _: () = assert!(offset_of!(CameraInstructionLayout, clear) == 0x148);
const _: () = assert!(size_of::<CameraInstructionLayout>() == 0x150);
