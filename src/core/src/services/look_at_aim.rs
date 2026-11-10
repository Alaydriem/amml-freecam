use crate::services::PathCurve;
use crate::services::path_store::PATH_CAPACITY;
use crate::structs::{CameraPose, PathKey, Vec3};

// camera-to-target length in blocks at or below which the camera keeps the v1 rotation
pub const HOLD_DISTANCE: f32 = 0.5;
// camera-to-target length in blocks from which the camera aims fully at the target
pub const BLEND_DISTANCE: f32 = 2.0;
// largest angle in degrees between the aim and the v1 rotation over a segment up to which the segment
// aims fully at the target
pub const AGREE_DEGREES: f32 = 45.0;
// largest angle in degrees from which the segment keeps the v1 rotation
pub const DISAGREE_DEGREES: f32 = 90.0;
// largest aim turn rate over a segment, as a multiple of the v1 rotation's turn rate, up to which the
// segment aims fully at the target
pub const RATE_RATIO_FULL: f32 = 1.25;
// largest aim turn rate multiple from which the segment keeps the v1 rotation
pub const RATE_RATIO_NONE: f32 = 2.0;
// degrees per block of travel that an aim may always turn, whatever the v1 rotation does
pub const RATE_FLOOR_DEGREES_PER_BLOCK: f32 = 10.0;
// samples per segment for the trust measurement
const TRUST_SAMPLES: usize = 64;
// share of a segment at each end over which its trust meets the trust of the key there
const TRUST_EDGE: f32 = 0.1;
// travel in blocks between trust samples below which the camera counts as standing still
const MIN_TRAVEL: f32 = 1e-5;
const PITCH_LIMIT: f32 = 89.9;
// share of the aim vector that must be horizontal for its yaw to be followed
const VERTICAL_SHARE: f32 = 1e-4;

// aims the camera at a target that moves along a curve through the look-at keys' targets
pub struct LookAtAim {
    // one control point per key: its stored target, or a synthetic one on its view ray
    targets: Vec<CameraPose>,
    // the key poses, which pace the target curve so it moves in step with the position curve
    poses: Vec<CameraPose>,
    // true for a key with a stored target
    stored: Vec<bool>,
    // one per segment: 1 where the aim follows the target, 0 where it keeps the v1 rotation
    trust: Vec<f32>,
    curve: PathCurve,
    any_target: bool,
    // the last yaw and pitch handed out; yaw is unwrapped against it
    last: Option<(f32, f32)>,
}

impl LookAtAim {
    pub fn new() -> Self {
        Self {
            targets: Vec::with_capacity(PATH_CAPACITY),
            poses: Vec::with_capacity(PATH_CAPACITY),
            stored: Vec::with_capacity(PATH_CAPACITY),
            trust: Vec::with_capacity(PATH_CAPACITY),
            curve: PathCurve::new(),
            any_target: false,
            last: None,
        }
    }

    // a key with no target gets one on its view ray, at the mean target distance of its look-at
    // neighbours, or of every look-at key when no neighbour has a target; `positions` is playback's camera
    // position curve through the same keys, used to measure the trust of each segment
    pub fn rebuild(&mut self, keys: &[PathKey], positions: &PathCurve) {
        let keys = &keys[..keys.len().min(PATH_CAPACITY)];
        self.targets.clear();
        self.poses.clear();
        self.stored.clear();
        self.trust.clear();
        self.last = None;
        let (sum, count) = keys
            .iter()
            .filter_map(Self::target_distance)
            .fold((0.0, 0u32), |(sum, count), distance| (sum + distance, count + 1));
        self.any_target = count > 0;
        if !self.any_target {
            self.curve.rebuild(&[]);
            return;
        }
        let mean = sum / count as f32;
        for (index, key) in keys.iter().enumerate() {
            let target = match key.target {
                Some(target) => target,
                None => key.pose.position + key.pose.forward() * Self::synthetic_distance(keys, index, mean),
            };
            self.stored.push(key.target.is_some());
            self.targets.push(CameraPose::new(target, 0.0, 0.0));
            self.poses.push(key.pose);
        }
        self.curve.rebuild_paced(&self.targets, &self.poses);
        for segment in 0..keys.len().saturating_sub(1) {
            let trust = self.measure_trust(positions, segment);
            self.trust.push(trust);
        }
    }

    // forgets the held aim, for a new playback
    pub fn reset(&mut self) {
        self.last = None;
    }

    // the pose at `position` aimed at the target for segment `segment` at local `t`; the fallback
    // rotation when neither end of the segment has a target
    pub fn aim(&mut self, position: Vec3, segment: usize, t: f32, fallback: CameraPose) -> CameraPose {
        let (yaw, pitch) = self.rotation(position, segment, t, fallback);
        self.last = Some((yaw, pitch));
        CameraPose::new(position, yaw, pitch)
    }

    // the aim blends toward the v1 rotation with two weights: the segment's trust, and a smoothstep on
    // the camera-to-target length (v1 at HOLD_DISTANCE, full aim at BLEND_DISTANCE), so a close
    // target can neither snap nor spin the camera
    fn rotation(&self, position: Vec3, segment: usize, t: f32, fallback: CameraPose) -> (f32, f32) {
        if !self.any_target || !self.segment_has_target(segment) {
            return (fallback.yaw, fallback.pitch);
        }
        let last_yaw = self.last.map_or(fallback.yaw, |(yaw, _)| yaw);
        let to_target = self.curve.position_in_segment(segment, t) - position;
        let length = to_target.length();
        let distance_weight =
            if length.is_finite() { Self::smoothstep(HOLD_DISTANCE, BLEND_DISTANCE, length) } else { 0.0 };
        let weight = distance_weight * self.trust_at(segment, t);
        let (yaw, pitch) = if weight <= 0.0 {
            (fallback.yaw, fallback.pitch)
        } else {
            let horizontal = (to_target.x * to_target.x + to_target.z * to_target.z).sqrt();
            let aim_pitch = (-to_target.y).atan2(horizontal).to_degrees().clamp(-PITCH_LIMIT, PITCH_LIMIT);
            let aim_yaw = if horizontal > length * VERTICAL_SHARE {
                (-to_target.x).atan2(to_target.z).to_degrees()
            } else {
                last_yaw
            };
            if weight >= 1.0 {
                (aim_yaw, aim_pitch)
            } else {
                let yaw = fallback.yaw + weight * PathCurve::wrap_degrees(aim_yaw - fallback.yaw);
                (yaw, fallback.pitch + weight * (aim_pitch - fallback.pitch))
            }
        };
        (last_yaw + PathCurve::wrap_degrees(yaw - last_yaw), pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT))
    }

    // the segment's trust inside it; near each end it meets the trust of the key there, the lower of
    // its two segments, so the weight has no step at a key
    fn trust_at(&self, segment: usize, t: f32) -> f32 {
        let Some(&own) = self.trust.get(segment) else {
            return 1.0;
        };
        let key = |index: usize| {
            let before = index.checked_sub(1).and_then(|before| self.trust.get(before)).copied().unwrap_or(own);
            let after = self.trust.get(index).copied().unwrap_or(own);
            before.min(after)
        };
        let start = 1.0 - Self::smoothstep(0.0, TRUST_EDGE, t);
        let end = Self::smoothstep(1.0 - TRUST_EDGE, 1.0, t);
        own + (key(segment) - own) * start + (key(segment + 1) - own) * end
    }

    // how far a segment's aim can be trusted: it falls with the largest angle between the aim and the
    // v1 rotation, and with the largest aim turn rate per block of travel as a multiple of the v1
    // rotation's (raised to RATE_FLOOR_DEGREES_PER_BLOCK); a target curve that passes by or behind
    // the camera fails both, and the segment keeps the v1 rotation
    fn measure_trust(&self, positions: &PathCurve, segment: usize) -> f32 {
        let mut largest_apart: f32 = 0.0;
        let mut largest_ratio: f32 = 0.0;
        let mut previous: Option<(Vec3, Vec3, Vec3)> = None;
        for step in 0..=TRUST_SAMPLES {
            let t = step as f32 / TRUST_SAMPLES as f32;
            let pose = positions.pose_in_segment(segment, t);
            let to_target = self.curve.position_in_segment(segment, t) - pose.position;
            let forward = pose.forward();
            if to_target.length() > HOLD_DISTANCE {
                largest_apart = largest_apart.max(Self::angle_degrees(to_target, forward));
            }
            if let Some((position, aim, view)) = previous {
                let travel = (pose.position - position).length();
                if travel > MIN_TRAVEL {
                    let aim_rate = Self::angle_degrees(aim, to_target) / travel;
                    let v1_rate = Self::angle_degrees(view, forward) / travel;
                    largest_ratio = largest_ratio.max(aim_rate / v1_rate.max(RATE_FLOOR_DEGREES_PER_BLOCK));
                }
            }
            previous = Some((pose.position, to_target, forward));
        }
        let agreement = 1.0 - Self::smoothstep(AGREE_DEGREES, DISAGREE_DEGREES, largest_apart);
        let calm = 1.0 - Self::smoothstep(RATE_RATIO_FULL, RATE_RATIO_NONE, largest_ratio);
        agreement * calm
    }

    // 0 at or below `low`, 1 at or past `high`, smoothstep between
    fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
        let x = ((value - low) / (high - low)).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    }

    // 0 when either vector has no direction
    fn angle_degrees(a: Vec3, b: Vec3) -> f32 {
        let cosine = a.normalized().dot(b.normalized());
        if cosine.is_finite() { cosine.clamp(-1.0, 1.0).acos().to_degrees() } else { 0.0 }
    }

    fn segment_has_target(&self, segment: usize) -> bool {
        let stored = |index: usize| self.stored.get(index).copied().unwrap_or(false);
        stored(segment) || stored(segment + 1)
    }

    fn target_distance(key: &PathKey) -> Option<f32> {
        key.target.map(|target| (target - key.pose.position).length())
    }

    fn synthetic_distance(keys: &[PathKey], index: usize, fallback: f32) -> f32 {
        let before = index.checked_sub(1).and_then(|before| keys.get(before));
        let after = keys.get(index + 1);
        let (sum, count) = [before, after]
            .into_iter()
            .flatten()
            .filter_map(Self::target_distance)
            .fold((0.0, 0u32), |(sum, count), distance| (sum + distance, count + 1));
        if count == 0 { fallback } else { sum / count as f32 }
    }
}

impl Default for LookAtAim {
    fn default() -> Self {
        Self::new()
    }
}
