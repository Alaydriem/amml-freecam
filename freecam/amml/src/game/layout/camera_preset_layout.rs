use amml_msvc::MsvcString;

// the game's SharedTypes::v1_26_50::CameraPreset (130h bytes): the name (the text of its Identifier) first, then the
// rest, which freecam does not read. `CameraPreset.size` checks the size
#[repr(C)]
pub struct CameraPresetLayout {
    pub name: MsvcString, // 0x000 mName
    _rest: [u64; 34],     // 0x020 mInheritFrom .. mStartingRot
}

const _: () = assert!(size_of::<CameraPresetLayout>() == 0x130);
