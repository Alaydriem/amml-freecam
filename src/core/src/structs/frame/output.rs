use crate::structs::CameraPose;
use crate::structs::frame::{CameraCommand, CaptureRequest, RayRequest};

// what the mod layer does this frame
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameOutput {
    pub camera: CameraCommand,
    pub render_override: Option<CameraPose>,
    // true while the UI is drawn
    pub ui_visible: bool,
    // the mod manager shows the cursor: bar mode is on and no right-button look drag holds the mouse
    pub cursor_mode: bool,
    // freecam wants its mod manager leases: from a granted start until it is fully off
    pub leases_wanted: bool,
    // true while freecam controls the camera: from turning on until fully off
    pub camera_owned: bool,
    // a ray to cast for a look-at key, answered with the controller's ray result
    pub ray: Option<RayRequest>,
    // true while a screenshot is under way: no mod drawing may reach the picture
    pub hide_ui: bool,
    // capture the frame presented now, answered with the controller's capture result
    pub capture: Option<CaptureRequest>,
}
