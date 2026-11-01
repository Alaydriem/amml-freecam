use crate::structs::{CameraPose, Vec3};

// the game state the mod layer reads once per frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStatus {
    pub time_seconds: f64,
    pub in_world: bool,
    pub menu_open: bool,
    pub focused: bool,
    // the camera handler is captured and `minecraft:free` is in the preset list
    pub ready: bool,
    // the active camera is first person, third person or third person front
    pub player_camera_active: bool,
    // the player's head position and rotation
    pub start_pose: CameraPose,
    pub health: i32,
    pub dimension: i32,
    pub rendered_position: Option<Vec3>,
    pub game_camera_forward: Option<Vec3>,
    pub game_camera_position: Option<Vec3>,
    // vertical field of view of the game camera, degrees
    pub fov_y_degrees: f32,
    // width over height of the viewport
    pub aspect: f32,
    // mod-layer time spent delivering the previous frame's marker batch, 0 when nothing was sent
    pub last_shape_send_us: f32,
    // count of camera handler captures, raised by one on each capture; 0 while none was captured
    pub handler_generation: u32,
    // count of presented frames, raised by one on each Present; the mod layer may call the frame call more than once per present
    pub present_count: u32,
    // another mod holds the Camera lease, so the player camera is that mod's to give back; read while exiting
    pub camera_held_by_other: bool,
}
