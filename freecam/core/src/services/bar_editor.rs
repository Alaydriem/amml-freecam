use crate::config::LookAtConfig;
use crate::services::{BarState, CameraMotion, PathSession, SmoothnessTelemetry};
use crate::structs::{CameraPose, KeyEdit, PathKey, RayRequest, UiAction, Vec3};

// how long a first Clear points click stays armed
pub const CLEAR_ARM_SECONDS: f64 = 3.0;
// frames after a look-at ray request before the key is stored without a target
const RAY_TIMEOUT_FRAMES: u32 = 2;

// the playback bar's edits to the path: adding, re-placing and deleting keys, selecting, scrubbing and
// rewinding, the Clear points arm, and the look-at key that waits for its ray
pub struct BarEditor {
    bar: BarState,
    look_at_enabled: bool,
    look_at_distance: f32,
    // the pose of a key that waits for its look-at ray
    pending_key: Option<CameraPose>,
    // what the pending key does to the path once it is stored
    pending_edit: KeyEdit,
    // frames since the pending key's ray went out, None until it goes out
    pending_frames: Option<u32>,
    // seconds the Clear points click stays armed, 0 when it is not
    clear_arm_left: f64,
}

impl BarEditor {
    pub fn new(look_at: LookAtConfig) -> Self {
        Self {
            bar: BarState::new(),
            look_at_enabled: look_at.enabled,
            look_at_distance: look_at.max_distance,
            pending_key: None,
            pending_edit: KeyEdit::Insert,
            pending_frames: None,
            clear_arm_left: 0.0,
        }
    }

    // a new look-at config; a key already waiting for its ray keeps the old one
    pub fn set_look_at(&mut self, look_at: LookAtConfig) {
        self.look_at_enabled = look_at.enabled;
        self.look_at_distance = look_at.max_distance;
    }

    pub fn selection(&self) -> Option<usize> {
        self.bar.selection()
    }

    pub fn playhead(&self) -> f32 {
        self.bar.playhead()
    }

    pub fn set_playhead(&mut self, distance: f32) {
        self.bar.set_playhead(distance);
    }

    pub fn clear_armed(&self) -> bool {
        self.clear_arm_left > 0.0
    }

    // counts the Clear points arm down by `dt` seconds
    pub fn tick(&mut self, dt: f64) {
        self.clear_arm_left = (self.clear_arm_left - dt).max(0.0);
    }

    pub fn disarm(&mut self) {
        self.clear_arm_left = 0.0;
    }

    // a bar click; true when it asks for playback to start
    pub fn apply(&mut self, action: UiAction, path: &mut PathSession, motion: &mut CameraMotion) -> bool {
        match action {
            UiAction::ClearPoints => {
                self.click_clear(path);
                false
            }
            UiAction::Screenshot => false,
            action => {
                // a key that waits for its ray is stored first, so indexes and the playhead are final
                self.store_pending(None, path);
                self.edit(action, path, motion)
            }
        }
    }

    fn edit(&mut self, action: UiAction, path: &mut PathSession, motion: &mut CameraMotion) -> bool {
        match action {
            UiAction::Play => return true,
            UiAction::Rewind => {
                self.bar.rewind();
                if !path.keys().is_empty() {
                    motion.reset(path.point_pose(0));
                }
            }
            UiAction::Scrub(distance) => {
                if path.keys().is_empty() {
                    return false;
                }
                self.bar.scrub(distance, path.curve().length());
                motion.reset(path.pose_at(self.bar.playhead()));
            }
            UiAction::SelectPoint(index) => self.select_point(index, path, motion),
            UiAction::Replace => {
                if let Some(index) = self.bar.selection() {
                    self.begin_key(KeyEdit::Replace(index), motion.pose(), path);
                }
            }
            UiAction::Delete => self.delete_selected(path),
            UiAction::AddPoint => self.begin_key(KeyEdit::Insert, motion.pose(), path),
            UiAction::ClearPoints | UiAction::Screenshot => {}
        }
        false
    }

    // the camera takes the pose of the point, look-at aim included, so flight continues from there
    fn select_point(&mut self, index: usize, path: &mut PathSession, motion: &mut CameraMotion) {
        if index >= path.keys().len() {
            return;
        }
        let distance = path.curve().point_distance(index);
        if self.bar.select(index, distance) {
            motion.reset(path.point_pose(index));
        }
    }

    // removes the selected point and moves the playhead to the point before it
    pub fn delete_selected(&mut self, path: &mut PathSession) {
        if self.bar.selection().is_none() {
            return;
        }
        self.store_pending(None, path);
        let Some(index) = self.bar.selection() else {
            return;
        };
        if path.delete_selected(Some(index)) {
            let distance = path.curve().point_distance(index.saturating_sub(1));
            self.bar.on_deleted(distance);
            self.clamp(path);
        }
    }

    // keeps the selection and the playhead inside the path
    pub fn clamp(&mut self, path: &PathSession) {
        self.bar.clamp(path.keys().len(), path.curve().length());
    }

    // the first click arms, a second within the arm time clears; only a path with points can arm
    fn click_clear(&mut self, path: &mut PathSession) {
        if path.points().is_empty() {
            return;
        }
        if self.clear_arm_left > 0.0 {
            self.clear_arm_left = 0.0;
            self.pending_key = None;
            self.pending_frames = None;
            path.clear_points();
            self.clamp(path);
            curia::info!("freecam path cleared by the user");
        } else {
            self.clear_arm_left = CLEAR_ARM_SECONDS;
        }
    }

    // the distance a playback starts from; a key that waits for its ray is stored first
    pub fn prepare_play(&mut self, path: &mut PathSession) -> f32 {
        self.store_pending(None, path);
        self.bar.play_from(path.curve().length())
    }

    // `pose` as a new or replacing key: a plain key is stored at once, a look-at key waits for the ray
    // from the camera
    pub fn begin_key(&mut self, edit: KeyEdit, pose: CameraPose, path: &mut PathSession) {
        self.store_pending(None, path);
        if !self.look_at_enabled {
            self.store_key(PathKey::new(pose, None), edit, path);
            return;
        }
        self.pending_key = Some(pose);
        self.pending_edit = edit;
    }

    // an insert goes after the last point at or before the playhead and moves the playhead to it; a
    // replace keeps the selection and moves the playhead to the replaced point
    fn store_key(&mut self, key: PathKey, edit: KeyEdit, path: &mut PathSession) {
        match edit {
            KeyEdit::Insert => {
                let index = self.bar.insert_index(path.curve(), path.keys().len());
                if path.insert(index, key) {
                    self.bar.on_inserted(index, path.curve().point_distance(index));
                }
            }
            KeyEdit::Replace(index) => {
                if path.replace(index, key) {
                    self.bar.set_playhead(path.curve().point_distance(index));
                }
            }
        }
    }

    // the answer to the pending key's ray: the hit position, or None when the ray hit nothing; an answer
    // with no request waiting changes nothing
    pub fn on_ray_result(&mut self, hit: Option<Vec3>, path: &mut PathSession, telemetry: &mut SmoothnessTelemetry) {
        if self.pending_frames.is_none() {
            return;
        }
        telemetry.record_look_at_ray(true);
        self.store_pending(hit.filter(|hit| hit.is_finite()), path);
    }

    // sends the pending key's ray once, and stores the key without a target when no answer comes in
    // time; a pending key is dropped when freecam is not `editing`
    pub fn update_pending(
        &mut self,
        editing: bool,
        path: &mut PathSession,
        telemetry: &mut SmoothnessTelemetry,
    ) -> Option<RayRequest> {
        let pose = self.pending_key?;
        if !editing {
            self.pending_key = None;
            self.pending_frames = None;
            return None;
        }
        let Some(frames) = self.pending_frames else {
            self.pending_frames = Some(0);
            return Some(RayRequest {
                origin: pose.position,
                direction: pose.forward(),
                max_distance: self.look_at_distance,
            });
        };
        let frames = frames.saturating_add(1);
        if frames >= RAY_TIMEOUT_FRAMES {
            telemetry.record_look_at_ray(false);
            self.store_pending(None, path);
        } else {
            self.pending_frames = Some(frames);
        }
        None
    }

    // stores the pending key, if any, with this target
    fn store_pending(&mut self, target: Option<Vec3>, path: &mut PathSession) {
        self.pending_frames = None;
        if let Some(pose) = self.pending_key.take() {
            self.store_key(PathKey::new(pose, target), self.pending_edit, path);
        }
    }
}
