use amml_api::{ApiError, Camera, CameraControl, Logger};
use freecam_core::structs::{CameraCommand, CameraPose};

use crate::services::PoseConvert;

// carries core's camera out through the loader: the free camera (`set_free` each frame while flying, so chunks load
// around it; `release_free` at the end) and the rendered pose (`set_pose`, which lives for one frame). The free
// camera gives `NotReady` until the loader has seen the client packet handler in this world; core asks again each
// frame, so the next frame tries again
pub struct CameraDriver {
    free_set: bool,
    // the error of the last refused free camera call, so a repeated refusal gives one line
    last_error: Option<ApiError>,
    pose_frames: u64,
}

impl CameraDriver {
    pub fn new() -> Self {
        Self { free_set: false, last_error: None, pose_frames: 0 }
    }

    // true while the free camera is set by this mod
    pub fn free_set(&self) -> bool {
        self.free_set
    }

    // the camera command of one core frame
    pub fn command(&mut self, control: &CameraControl, command: CameraCommand, log: &Logger) {
        match command {
            CameraCommand::Set(pose) => match control.set_free(PoseConvert::to_api(pose)) {
                Ok(()) => {
                    self.last_error = None;
                    if !self.free_set {
                        self.free_set = true;
                        log.info("freecam on", &[("free_camera", "set")]);
                    }
                }
                Err(error) => self.refused("set_free", error, log),
            },
            CameraCommand::Clear => {
                let result = control.release_free();
                let was_set = std::mem::replace(&mut self.free_set, false);
                match result {
                    Ok(()) if was_set => log.info("freecam off", &[("free_camera", "released")]),
                    Ok(()) => {}
                    Err(error) => self.refused("release_free", error, log),
                }
            }
            CameraCommand::None => {}
        }
    }

    // the pose of this frame, or none when freecam does not fly. One line at the first applied frame of a flight and
    // one with the count when the flight ends
    pub fn set_pose(&mut self, camera: &Camera<'_>, pose: Option<CameraPose>, log: &Logger) {
        let Some(pose) = pose else {
            if self.pose_frames > 0 {
                log.info("camera pose applied", &[("frames", &self.pose_frames.to_string()), ("flight", "ended")]);
                self.pose_frames = 0;
            }
            return;
        };
        if camera.set_pose(PoseConvert::to_api(pose)).is_ok() {
            self.pose_frames += 1;
            if self.pose_frames == 1 {
                log.info("camera pose applied", &[("frames", "1")]);
            }
        }
    }

    // a world leave: the loader forgot the free camera
    pub fn forget(&mut self) {
        self.free_set = false;
        self.last_error = None;
        self.pose_frames = 0;
    }

    fn refused(&mut self, call: &str, error: ApiError, log: &Logger) {
        if self.last_error == Some(error) {
            return;
        }
        self.last_error = Some(error);
        log.info(
            "free camera call refused; the next frame tries again",
            &[("call", call), ("error", &error.to_string())],
        );
    }
}

impl Default for CameraDriver {
    fn default() -> Self {
        Self::new()
    }
}
