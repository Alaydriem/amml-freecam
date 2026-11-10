use crate::services::path_store::PATH_CAPACITY;
use crate::structs::CameraPose;
use crate::structs::math::Vec3;
use crate::structs::path::CurveSample;

pub const SAMPLES_PER_SEGMENT: usize = 32;

const KNOT_FLOOR: f32 = 1e-4;
// a pace interval below this, pace points under 0.01 blocks apart, gives way to the curve's own interval
const MIN_PACE_INTERVAL: f32 = 0.1;
const PITCH_LIMIT: f32 = 89.9;

pub struct PathCurve {
    // control points with yaw unwrapped into one continuous run
    points: Vec<CameraPose>,
    // knot interval of each segment, from the pace points
    intervals: Vec<f32>,
    samples: Vec<CurveSample>,
}

impl PathCurve {
    pub fn new() -> Self {
        Self {
            points: Vec::with_capacity(PATH_CAPACITY),
            intervals: Vec::with_capacity(PATH_CAPACITY),
            samples: Vec::with_capacity(PATH_CAPACITY * SAMPLES_PER_SEGMENT + 1),
        }
    }

    // reuses the buffers made by new; points past PATH_CAPACITY are ignored
    pub fn rebuild(&mut self, points: &[CameraPose]) {
        self.rebuild_paced(points, points);
    }

    // the same as rebuild, but the knot spacing comes from the positions of `pace`, one per point, so
    // this curve sampled at another curve's segment and local t moves in step with it; a pace of
    // another length is ignored, and where two pace points almost coincide the curve's own spacing
    // is used so the curve cannot overshoot
    pub fn rebuild_paced(&mut self, points: &[CameraPose], pace: &[CameraPose]) {
        let pace = if pace.len() == points.len() { pace } else { points };
        self.intervals.clear();
        let count = points.len().min(PATH_CAPACITY);
        self.intervals.extend(pace[..count].windows(2).zip(points[..count].windows(2)).map(|(paced, own)| {
            let interval = Self::knot_interval(paced[0].position, paced[1].position);
            if interval < MIN_PACE_INTERVAL { Self::knot_interval(own[0].position, own[1].position) } else { interval }
        }));
        self.points.clear();
        self.samples.clear();
        let mut previous_yaw = 0.0;
        for (index, point) in points.iter().take(PATH_CAPACITY).enumerate() {
            let yaw = if index == 0 { point.yaw } else { previous_yaw + Self::wrap_degrees(point.yaw - previous_yaw) };
            previous_yaw = yaw;
            self.points.push(CameraPose::new(point.position, yaw, point.pitch));
        }
        self.fill_samples();
    }

    pub fn length(&self) -> f32 {
        self.samples.last().map_or(0.0, |sample| sample.distance)
    }

    // distance along the curve at which control point `index` sits
    pub fn point_distance(&self, index: usize) -> f32 {
        let last = self.points.len().saturating_sub(1);
        if index >= last {
            return self.length();
        }
        self.samples.get(index * SAMPLES_PER_SEGMENT).map_or(0.0, |sample| sample.distance)
    }

    // capacity of the arc-length table
    pub fn sample_capacity(&self) -> usize {
        self.samples.capacity()
    }

    pub fn pose_at(&self, distance: f32) -> CameraPose {
        let (segment, t) = self.segment_at(distance);
        self.pose_in_segment(segment, t)
    }

    // the segment index and local parameter at this distance along the curve; (0, 0.0) before the start
    // or on a curve with no length
    pub fn segment_at(&self, distance: f32) -> (usize, f32) {
        let segments = self.points.len().saturating_sub(1);
        let length = self.length();
        if segments == 0 || self.samples.len() < 2 || length <= 0.0 || distance.is_nan() || distance <= 0.0 {
            return (0, 0.0);
        }
        if distance >= length {
            return (segments - 1, 1.0);
        }
        let upper = self.samples.partition_point(|sample| sample.distance <= distance).clamp(1, self.samples.len() - 1);
        let low = self.samples[upper - 1];
        let high = self.samples[upper];
        let span = high.distance - low.distance;
        let fraction = if span > 0.0 { ((distance - low.distance) / span).clamp(0.0, 1.0) } else { 0.0 };
        let high_t = if high.segment == low.segment { high.t } else { 1.0 };
        (low.segment as usize, low.t + (high_t - low.t) * fraction)
    }

    // the pose at local parameter `t` of segment `index`; the index is clamped to the last segment and
    // the ends of a segment are its control points exactly
    pub fn pose_in_segment(&self, index: usize, t: f32) -> CameraPose {
        let count = self.points.len();
        if count < 2 {
            return self.points.first().map_or(CameraPose::default(), |first| Self::clamped(*first));
        }
        let index = index.min(count - 2);
        if t.is_nan() || t <= 0.0 {
            return Self::clamped(self.points[index]);
        }
        if t >= 1.0 {
            return Self::clamped(self.points[index + 1]);
        }
        self.evaluate(index, t)
    }

    pub fn position_in_segment(&self, index: usize, t: f32) -> Vec3 {
        self.pose_in_segment(index, t).position
    }

    pub fn position_at(&self, distance: f32) -> Vec3 {
        self.pose_at(distance).position
    }

    // refills `out` with `per_segment` positions per segment plus the final point
    pub fn sample_positions(&self, per_segment: usize, out: &mut Vec<Vec3>) {
        out.clear();
        let Some(last) = self.points.last() else {
            return;
        };
        let per_segment = per_segment.max(1);
        for segment in 0..self.points.len().saturating_sub(1) {
            for step in 0..per_segment {
                out.push(self.evaluate(segment, step as f32 / per_segment as f32).position);
            }
        }
        out.push(last.position);
    }

    fn fill_samples(&mut self) {
        let segments = self.points.len().saturating_sub(1);
        if segments == 0 {
            return;
        }
        let mut distance = 0.0;
        let mut previous = self.points[0].position;
        for segment in 0..segments {
            for step in 0..SAMPLES_PER_SEGMENT {
                let t = step as f32 / SAMPLES_PER_SEGMENT as f32;
                self.samples.push(CurveSample { distance, segment: segment as u32, t });
                let next_t = (step + 1) as f32 / SAMPLES_PER_SEGMENT as f32;
                let next = self.evaluate(segment, next_t).position;
                distance += (next - previous).length();
                previous = next;
            }
        }
        self.samples.push(CurveSample { distance, segment: (segments - 1) as u32, t: 1.0 });
    }

    fn evaluate(&self, segment: usize, t: f32) -> CameraPose {
        let count = self.points.len();
        let p1 = self.points[segment];
        let p2 = self.points[segment + 1];
        let p0 = if segment == 0 { Self::mirror(p1, p2) } else { self.points[segment - 1] };
        let p3 = if segment + 2 >= count { Self::mirror(p2, p1) } else { self.points[segment + 2] };

        let positions = [p0.position, p1.position, p2.position, p3.position];
        let middle = self.intervals[segment];
        let before = if segment == 0 { middle } else { self.intervals[segment - 1] };
        let after = self.intervals.get(segment + 1).copied().unwrap_or(middle);
        let k1 = before;
        let k2 = k1 + middle;
        let k3 = k2 + after;
        let position = Self::barry_goldman(positions, [0.0, k1, k2, k3], k1 + t * (k2 - k1));

        let angles = [p0, p1, p2, p3].map(|pose| Vec3::new(pose.yaw, pose.pitch, 0.0));
        let angle = Self::barry_goldman(angles, [0.0, 1.0, 2.0, 3.0], 1.0 + t);
        Self::clamped(CameraPose::new(position, angle.x, angle.y))
    }

    // phantom point past `from`, reflecting `toward` across it
    fn mirror(from: CameraPose, toward: CameraPose) -> CameraPose {
        CameraPose::new(
            from.position * 2.0 - toward.position,
            from.yaw * 2.0 - toward.yaw,
            from.pitch * 2.0 - toward.pitch,
        )
    }

    fn knot_interval(from: Vec3, to: Vec3) -> f32 {
        (to - from).length().sqrt().max(KNOT_FLOOR)
    }

    fn barry_goldman(p: [Vec3; 4], k: [f32; 4], t: f32) -> Vec3 {
        let a1 = Self::lerp(p[0], p[1], (t - k[0]) / (k[1] - k[0]));
        let a2 = Self::lerp(p[1], p[2], (t - k[1]) / (k[2] - k[1]));
        let a3 = Self::lerp(p[2], p[3], (t - k[2]) / (k[3] - k[2]));
        let b1 = Self::lerp(a1, a2, (t - k[0]) / (k[2] - k[0]));
        let b2 = Self::lerp(a2, a3, (t - k[1]) / (k[3] - k[1]));
        Self::lerp(b1, b2, (t - k[1]) / (k[2] - k[1]))
    }

    fn lerp(from: Vec3, to: Vec3, fraction: f32) -> Vec3 {
        from + (to - from) * fraction
    }

    fn clamped(pose: CameraPose) -> CameraPose {
        CameraPose::new(pose.position, pose.yaw, pose.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT))
    }

    // wraps an angle difference into [-180, 180]
    pub fn wrap_degrees(delta: f32) -> f32 {
        delta - 360.0 * (delta / 360.0).round()
    }
}

impl Default for PathCurve {
    fn default() -> Self {
        Self::new()
    }
}
