use std::ffi::c_void;
use std::sync::{Mutex, MutexGuard, PoisonError};

use amml_game::CameraWriter;
use freecam_core::structs::{CameraPose, Vec3};

use crate::types::{LookBasis, OverrideState};

// the free camera pose the render hooks write over the game's camera: the rotation after setupCamera, the position in
// the view object and in the level camera that sorts the chunks
pub struct RenderOverride {
    state: Mutex<OverrideState>,
}

impl RenderOverride {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(OverrideState {
                active: false,
                pose: CameraPose { position: Vec3 { x: 0.0, y: 0.0, z: 0.0 }, yaw: 0.0, pitch: 0.0 },
                rendered: None,
            }),
        }
    }

    // once per frame, from core's output
    pub fn set(&self, pose: Option<CameraPose>) {
        let mut state = self.lock();
        state.active = pose.is_some();
        if let Some(pose) = pose {
            state.pose = pose;
        }
    }

    pub fn pose(&self) -> Option<CameraPose> {
        let state = self.lock();
        state.active.then_some(state.pose)
    }

    /// after setupCamera: the view looks along the pose's yaw and pitch
    ///
    /// # Safety
    /// `camera` must be null or the live Camera of the setupCamera call that just returned
    #[allow(unsafe_code)]
    pub unsafe fn apply_rotation(&self, writer: &CameraWriter, camera: *mut c_void) {
        let Some(pose) = self.pose() else { return };
        let basis = LookBasis::from_angles(pose.yaw, pose.pitch);
        // SAFETY: forwarded to the caller
        unsafe { writer.write_rotation(camera, basis.forward, basis.right, basis.up, basis.view, basis.inverse) };
    }

    /// after createViewRenderObject: the frame is drawn from the pose's position
    ///
    /// # Safety
    /// `view_object` must be null or the ViewRenderObject the game just filled
    #[allow(unsafe_code)]
    pub unsafe fn apply_view(&self, writer: &CameraWriter, view_object: *mut c_void) {
        let Some(position) = self.take_position() else { return };
        // SAFETY: forwarded to the caller
        unsafe { writer.write_view_position(view_object, position) };
    }

    /// after LevelRendererCamera::preRenderUpdate: chunks sort around the pose's position
    ///
    /// # Safety
    /// `level_camera` must be null or the live LevelRendererCamera of the call that just returned
    #[allow(unsafe_code)]
    pub unsafe fn apply_level(&self, writer: &CameraWriter, level_camera: *mut c_void) {
        let Some(position) = self.take_position() else { return };
        // SAFETY: forwarded to the caller
        unsafe { writer.write_level_camera_position(level_camera, position) };
    }

    // the position the render hooks last wrote, once
    pub fn take_rendered(&self) -> Option<Vec3> {
        self.lock().rendered.take()
    }

    pub fn reset(&self) {
        let mut state = self.lock();
        state.active = false;
        state.rendered = None;
    }

    // the active position, kept as the rendered one
    fn take_position(&self) -> Option<[f32; 3]> {
        let mut state = self.lock();
        if !state.active {
            return None;
        }
        state.rendered = Some(state.pose.position);
        let position = state.pose.position;
        Some([position.x, position.y, position.z])
    }

    fn lock(&self) -> MutexGuard<'_, OverrideState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for RenderOverride {
    fn default() -> Self {
        Self::new()
    }
}
