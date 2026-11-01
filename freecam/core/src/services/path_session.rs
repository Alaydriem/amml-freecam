use std::time::Instant;

use crate::services::path_player::EASE_RAMP_SECONDS;
use crate::services::path_store::PATH_CAPACITY;
use crate::services::{LookAtAim, PathCurve, PathPlayer, PathStore};
use crate::structs::path::PlaybackStep;
use crate::structs::{CameraPose, PathKey};

const MIN_PLAY_POINTS: usize = 2;

// the recorded path for one world session, its curve and its playback
pub struct PathSession {
    store: PathStore,
    // the key poses, kept in step with the store so the curve rebuilds without allocating
    poses: Vec<CameraPose>,
    curve: PathCurve,
    aim: LookAtAim,
    player: PathPlayer,
    pitch_limit: f32,
    settle_frames: u32,
    was_in_world: bool,
    dimension: i32,
    // microseconds spent in curve and aim rebuilds since the last `take_rebuild_us`
    rebuild_us: f32,
}

impl PathSession {
    pub fn new(pitch_limit: f32, settle_frames: u32) -> Self {
        Self {
            store: PathStore::new(PATH_CAPACITY),
            poses: Vec::with_capacity(PATH_CAPACITY),
            curve: PathCurve::new(),
            aim: LookAtAim::new(),
            player: PathPlayer::new(EASE_RAMP_SECONDS),
            pitch_limit: pitch_limit.abs(),
            settle_frames,
            was_in_world: false,
            dimension: 0,
            rebuild_us: 0.0,
        }
    }

    pub fn set_pitch_limit(&mut self, pitch_limit: f32) {
        self.pitch_limit = pitch_limit.abs();
    }

    pub fn keys(&self) -> &[PathKey] {
        self.store.keys()
    }

    // the pose of every key, in path order
    pub fn points(&self) -> &[CameraPose] {
        &self.poses
    }

    // grows on every change to the keys
    pub fn revision(&self) -> u64 {
        self.store.revision()
    }

    pub fn curve(&self) -> &PathCurve {
        &self.curve
    }

    // puts the key at `index`, or at the end when the index is past it; false when the path is full
    pub fn insert(&mut self, index: usize, key: PathKey) -> bool {
        if !self.store.insert_at(index, key) {
            curia::info!("freecam path full", { capacity: PATH_CAPACITY });
            return false;
        }
        self.rebuild_curve();
        true
    }

    // false, with the path untouched, when the index is past the end
    pub fn replace(&mut self, index: usize, key: PathKey) -> bool {
        let replaced = self.store.replace_at(index, key);
        if replaced {
            self.rebuild_curve();
        }
        replaced
    }

    // removes the selected point; false, with the path untouched, when nothing is selected or the index is past the end
    pub fn delete_selected(&mut self, selected: Option<usize>) -> bool {
        let Some(index) = selected else {
            return false;
        };
        let removed = self.store.remove_at(index);
        if removed {
            self.rebuild_curve();
        }
        removed
    }

    // removes every point; false when the path was already empty
    pub fn clear_points(&mut self) -> bool {
        if self.store.is_empty() {
            return false;
        }
        self.store.clear();
        self.rebuild_curve();
        true
    }

    // the time a playback at this speed takes, 0 when the path cannot play
    pub fn shot_seconds(&self, speed: f32) -> f32 {
        if self.store.len() < MIN_PLAY_POINTS || speed <= 0.0 {
            return 0.0;
        }
        PathPlayer::duration(self.curve.length(), speed, EASE_RAMP_SECONDS)
    }

    // clears the path when the player leaves the world or changes dimension inside one
    pub fn observe_world(&mut self, in_world: bool, dimension: i32) {
        let left = self.was_in_world && !in_world;
        let moved = self.was_in_world && in_world && dimension != self.dimension;
        self.was_in_world = in_world;
        self.dimension = dimension;
        if !left && !moved {
            return;
        }
        if !self.store.is_empty() {
            curia::info!("freecam path cleared", { reason: if left { "left the world" } else { "dimension change" } });
        }
        self.player.stop();
        self.store.clear();
        self.rebuild_curve();
    }

    // the playback time at which a playback from the start reaches `distance`, the same mapping
    // playback uses; 0 when the path cannot play
    pub fn seconds_at(&self, distance: f32, speed: f32) -> f32 {
        if self.store.len() < MIN_PLAY_POINTS {
            return 0.0;
        }
        PathPlayer::seconds_at(self.curve.length(), speed, EASE_RAMP_SECONDS, distance)
    }

    // starts playback `from` blocks along the curve; returns false, and changes nothing, when the
    // path has too few points to play
    pub fn start(&mut self, speed: f32, from: f32) -> bool {
        if self.store.len() < MIN_PLAY_POINTS {
            curia::info!("freecam play needs more points", { needed: MIN_PLAY_POINTS, points: self.store.len() });
            return false;
        }
        self.player.start(self.curve.length(), speed, self.settle_frames, from);
        self.aim.reset();
        true
    }

    // the pose to show this frame, aimed at the look-at target where the segment has one, with pitch
    // kept inside the configured limit
    pub fn step(&mut self, dt: f64, paused: bool) -> (CameraPose, PlaybackStep) {
        let step = self.player.step(dt, paused);
        let (segment, t) = self.curve.segment_at(step.distance);
        (self.aimed(segment, t), step)
    }

    // the pose playback shows at `distance` along the curve, look-at aim included; a still pose
    // outside playback, so the aim starts fresh
    pub fn pose_at(&mut self, distance: f32) -> CameraPose {
        let (segment, t) = self.curve.segment_at(distance);
        self.fresh_aimed(segment, t)
    }

    // the pose playback shows at key `index`, look-at aim included; the last key for an index past it
    pub fn point_pose(&mut self, index: usize) -> CameraPose {
        let last = self.store.len().saturating_sub(1);
        let index = index.min(last);
        let (segment, t) = if index == last && index > 0 { (index - 1, 1.0) } else { (index, 0.0) };
        self.fresh_aimed(segment, t)
    }

    // the microseconds spent rebuilding the curve and the aim since the last call, then starts again from 0
    pub fn take_rebuild_us(&mut self) -> f32 {
        std::mem::take(&mut self.rebuild_us)
    }

    // the next step stays in the settle when playback has not started moving
    pub fn hold_settle(&mut self) {
        self.player.hold_settle();
    }

    pub fn stop(&mut self) {
        self.player.stop();
    }

    fn fresh_aimed(&mut self, segment: usize, t: f32) -> CameraPose {
        self.aim.reset();
        let pose = self.aimed(segment, t);
        self.aim.reset();
        pose
    }

    fn aimed(&mut self, segment: usize, t: f32) -> CameraPose {
        let along = self.curve.pose_in_segment(segment, t);
        let pose = self.aim.aim(along.position, segment, t, along);
        let pitch = pose.pitch.clamp(-self.pitch_limit, self.pitch_limit);
        CameraPose::new(pose.position, pose.yaw, pitch)
    }

    fn rebuild_curve(&mut self) {
        let started = Instant::now();
        self.poses.clear();
        self.poses.extend(self.store.keys().iter().map(|key| key.pose));
        self.curve.rebuild(&self.poses);
        self.aim.rebuild(self.store.keys(), &self.curve);
        self.rebuild_us += started.elapsed().as_secs_f32() * 1e6;
    }
}
