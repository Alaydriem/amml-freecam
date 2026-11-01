use freecam_core::structs::{CameraPose, Vec3};

// the free camera pose the render hooks write, and the position they last wrote
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OverrideState {
    pub active: bool,
    pub pose: CameraPose,
    pub rendered: Option<Vec3>,
}
