use freecam_core::structs::{CameraPose, FrameStatus, Vec3};

pub struct Frames;

impl Frames {
    pub fn start_pose() -> CameraPose {
        CameraPose::new(Vec3::new(10.0, 64.0, -5.0), 90.0, 10.0)
    }

    // one present per millisecond of test time, so every test frame at a new time is a new present
    pub fn present_count(time_seconds: f64) -> u32 {
        (time_seconds * 1000.0).round() as u32
    }

    // in world, focused, ready, handler captured once, player camera active, health 20, dimension 0
    pub fn status(time_seconds: f64) -> FrameStatus {
        FrameStatus {
            time_seconds,
            in_world: true,
            menu_open: false,
            focused: true,
            ready: true,
            player_camera_active: true,
            start_pose: Self::start_pose(),
            health: 20,
            dimension: 0,
            rendered_position: None,
            game_camera_forward: None,
            game_camera_position: None,
            fov_y_degrees: 70.0,
            aspect: 16.0 / 9.0,
            last_shape_send_us: 0.0,
            handler_generation: 1,
            present_count: Self::present_count(time_seconds),
            camera_held_by_other: false,
        }
    }
}
