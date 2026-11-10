use amml_msvc::MsvcVec;

use crate::game::layout::CameraPresetLayout;

// the game's CameraPresets (30h bytes): the preset vector, then the camera lists
#[repr(C)]
pub struct CameraPresetsLayout {
    pub presets: MsvcVec<CameraPresetLayout>, // 0x00 mPresets
    _lists: [usize; 3],                       // 0x18 mLists
}

const _: () = assert!(size_of::<CameraPresetsLayout>() == 0x30);
