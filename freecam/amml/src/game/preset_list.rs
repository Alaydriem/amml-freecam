use std::ffi::c_void;

use amml_game::GameEntries;
use amml_game::game::RawObject;

use crate::game::layout::{CameraPresetLayout, CameraPresetsLayout};
use crate::types::FreecamEntry;

// the camera presets the level holds, reached through `Level::getCameraPresets` and read through the CameraPresets
// mirror; `CameraPreset.size` checks the preset mirror
#[derive(Debug, Clone, Copy)]
pub struct PresetList {
    get_presets: u64,
}

impl PresetList {
    // the preset freecam asks the game to use
    pub const FREE_PRESET: &'static str = "minecraft:free";

    pub fn new(entries: &GameEntries) -> Option<Self> {
        let fits = entries.get(FreecamEntry::CAMERA_PRESET_SIZE)? == size_of::<CameraPresetLayout>() as u64;
        fits.then(|| Some(Self { get_presets: entries.get(FreecamEntry::LEVEL_GET_CAMERA_PRESETS)? })).flatten()
    }

    /// the index of `name` in the level's preset list; None when it is not there or a pointer is null. Game thread
    ///
    /// # Safety
    /// `level` must be null or the live Level (ILevel) of the local player
    #[allow(unsafe_code)]
    pub unsafe fn index_of(&self, level: *mut c_void, name: &str) -> Option<u32> {
        if level.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live Level, whose vtable holds getCameraPresets at the entry's slot
        let function = unsafe { RawObject::slot(level, self.get_presets) }?;
        // SAFETY: `CameraPresets const& Level::getCameraPresets() const`
        let presets = unsafe { RawObject::call_pointer(function, level) }?;
        // SAFETY: the level's live CameraPresets, laid out as the mirror
        let presets = unsafe { &*presets.cast::<CameraPresetsLayout>() };
        let index = presets.presets.as_slice().iter().position(|preset| preset.name.as_bytes() == name.as_bytes())?;
        u32::try_from(index).ok()
    }
}
