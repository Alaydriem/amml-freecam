use crate::config::MarkerConfig;
use crate::services::PathCurve;
use crate::structs::math::Vec3;
use crate::structs::{MarkerKind, MarkerShape, PathKey};

pub const MARKER_BLUE: u32 = 0xFF3D_A5FF;
pub const MARKER_GREEN: u32 = 0xFF4B_E35A;

const DASH_LENGTH: f32 = 0.5;
const DASH_GAP: f32 = 0.35;
const BOLD_OFFSET: f32 = 0.03;
const TARGET_BOX_SIZE: f32 = 0.25;
const ARROW_LENGTH: f32 = 1.0;
const TANGENT_STEP: f32 = 0.05;
const FALLBACK_FOV_DEGREES: f32 = 70.0;
const FALLBACK_ASPECT: f32 = 16.0 / 9.0;
const MAX_FOV_DEGREES: f32 = 180.0;
const BOX_EDGES: usize = 8;
const MIN_DIRECTION: f32 = 1e-4;
const WORLD_UP: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };

// builds the engine-drawn shapes for a path: a camera box per key, a look-at target box per look-at key,
// curve dashes and one arrow per segment, inside the `max_shapes` budget
pub struct MarkerBuilder {
    config: MarkerConfig,
    // sized once from `max_shapes` and never grown
    shapes: Vec<MarkerShape>,
    reductions: u32,
    // key count and curve length of the path the last budget lines were logged for
    logged_path: Option<(usize, u32)>,
}

impl MarkerBuilder {
    pub fn new(config: MarkerConfig) -> Self {
        let shapes = Vec::with_capacity(config.max_shapes);
        Self { config, shapes, reductions: 0, logged_path: None }
    }

    // refills the shape buffer; the selected key's box is green, or the last key's when nothing valid is
    // selected; the slice holds at most `max_shapes` shapes and stays valid until the next build
    pub fn build(
        &mut self,
        keys: &[PathKey],
        curve: &PathCurve,
        selected: Option<usize>,
        fov_y_degrees: f32,
        aspect: f32,
    ) -> &[MarkerShape] {
        self.shapes.clear();
        self.reductions = 0;
        if !self.config.enabled || keys.is_empty() {
            return &self.shapes;
        }
        let (weight, dashes, arrows) = self.budget(keys, curve);
        let (half_width, half_height) = Self::far_half_extents(self.config.frustum_depth, fov_y_degrees, aspect);
        let green = selected.filter(|index| *index < keys.len()).unwrap_or(keys.len() - 1);
        for (index, key) in keys.iter().enumerate() {
            let color = if index == green { MARKER_GREEN } else { MARKER_BLUE };
            self.push_key(key, color, weight, half_width, half_height);
        }
        self.push_dashes(curve, dashes);
        if arrows {
            self.push_arrows(keys, curve);
        }
        &self.shapes
    }

    // reduction steps applied in the last build: line weight, then fewer dashes, then no arrows
    pub fn reductions(&self) -> u32 {
        self.reductions
    }

    // picks line weight, dash count and arrows so the shapes fit `max_shapes`, reducing in order;
    // each reduction logs one line per path shape
    fn budget(&mut self, keys: &[PathKey], curve: &PathCurve) -> (usize, usize, bool) {
        let max = self.config.max_shapes;
        let segments = keys.len() - 1;
        let full_dashes = Self::full_dash_count(segments, curve.length());
        let targets = keys.iter().filter(|key| key.target.is_some()).count();
        let fixed = |weight: usize| keys.len() * BOX_EDGES * weight + targets;
        let path = (keys.len(), curve.length().to_bits());
        let log = self.logged_path != Some(path);

        let mut weight = self.config.line_weight.clamp(1, 2) as usize;
        let mut dashes = full_dashes;
        let mut arrows = segments;
        if fixed(weight) + dashes + arrows > max && weight > 1 {
            weight = 1;
            self.reductions += 1;
            if log {
                curia::info!("freecam markers: line weight 1 to fit the shape budget", { max: max });
            }
        }
        if fixed(weight) + dashes + arrows > max {
            self.reductions += 1;
            let room = max.saturating_sub(fixed(weight) + arrows);
            dashes = room;
            if log {
                curia::info!("freecam markers: curve dashes cut to fit the shape budget", { dashes: full_dashes, kept: room, max: max });
            }
            if room < segments.min(full_dashes) {
                self.reductions += 1;
                arrows = 0;
                dashes = max.saturating_sub(fixed(weight)).min(full_dashes);
                if log {
                    curia::info!("freecam markers: arrows dropped to fit the shape budget", { max: max });
                }
            }
        }
        let needed = fixed(weight) + dashes + arrows;
        if log && needed > max {
            curia::warn!("freecam markers: more shapes needed than drawn", { needed: needed, max: max });
        }
        if self.reductions > 0 {
            self.logged_path = Some(path);
        }
        (weight, dashes, arrows > 0)
    }

    fn full_dash_count(segments: usize, length: f32) -> usize {
        if segments == 0 || length <= 0.0 {
            return 0;
        }
        (length / (DASH_LENGTH + DASH_GAP)).ceil() as usize
    }

    // half-width and half-height of a box's far face, with fallbacks for a missing FOV or aspect
    fn far_half_extents(depth: f32, fov_y_degrees: f32, aspect: f32) -> (f32, f32) {
        let fov_valid = fov_y_degrees.is_finite() && fov_y_degrees > 0.0 && fov_y_degrees < MAX_FOV_DEGREES;
        let fov = if fov_valid { fov_y_degrees } else { FALLBACK_FOV_DEGREES };
        let aspect = if aspect.is_finite() && aspect > 0.0 { aspect } else { FALLBACK_ASPECT };
        let half_height = depth * (fov / 2.0).to_radians().tan();
        (half_height * aspect, half_height)
    }

    // the box edges of one key, their bold copies, then its target box
    fn push_key(&mut self, key: &PathKey, color: u32, weight: usize, half_width: f32, half_height: f32) {
        let apex = key.pose.position;
        let (forward, right, up) = Self::box_axes(key);
        let centre = apex + forward * self.config.frustum_depth;
        let corners = [
            centre + right * half_width + up * half_height,
            centre - right * half_width + up * half_height,
            centre - right * half_width - up * half_height,
            centre + right * half_width - up * half_height,
        ];
        let mut edges = [(apex, apex); BOX_EDGES];
        for (index, corner) in corners.iter().enumerate() {
            edges[index] = (apex, *corner);
            edges[index + 4] = (*corner, corners[(index + 1) % 4]);
        }
        for copy in 0..weight {
            for (start, end) in edges {
                let offset = if copy == 0 { Vec3::default() } else { Self::bold_offset(end - start, forward, right) };
                self.push(MarkerShape::new(MarkerKind::Line, start + offset, end + offset, color));
            }
        }
        if let Some(target) = key.target {
            self.push_target(target, color);
        }
    }

    // a small box centred on the target, with no line to the key
    fn push_target(&mut self, target: Vec3, color: u32) {
        let half = TARGET_BOX_SIZE / 2.0;
        let size = Vec3::new(TARGET_BOX_SIZE, TARGET_BOX_SIZE, TARGET_BOX_SIZE);
        self.push(MarkerShape::new(MarkerKind::Box, target - Vec3::new(half, half, half), size, color));
    }

    // dashes along the curve by arc length; fewer than the full count spreads them with longer gaps
    fn push_dashes(&mut self, curve: &PathCurve, count: usize) {
        let length = curve.length();
        if count == 0 || length <= 0.0 {
            return;
        }
        let full = (length / (DASH_LENGTH + DASH_GAP)).ceil() as usize;
        let step = if count >= full { DASH_LENGTH + DASH_GAP } else { length / count as f32 };
        for index in 0..count {
            let start = index as f32 * step;
            let end = (start + DASH_LENGTH).min(length);
            self.push(MarkerShape::new(
                MarkerKind::Line,
                curve.position_at(start),
                curve.position_at(end),
                MARKER_BLUE,
            ));
        }
    }

    // one arrow per segment, centred at its arc-length middle and pointing along the curve
    fn push_arrows(&mut self, keys: &[PathKey], curve: &PathCurve) {
        for segment in 0..keys.len() - 1 {
            let middle = (curve.point_distance(segment) + curve.point_distance(segment + 1)) / 2.0;
            let centre = curve.position_at(middle);
            let ahead = curve.position_at(middle + TANGENT_STEP) - curve.position_at(middle - TANGENT_STEP);
            let chord = keys[segment + 1].pose.position - keys[segment].pose.position;
            let tangent = if ahead.length() > MIN_DIRECTION { ahead.normalized() } else { chord.normalized() };
            let half = tangent * (ARROW_LENGTH / 2.0);
            self.push(MarkerShape::new(MarkerKind::Arrow, centre - half, centre + half, MARKER_BLUE));
        }
    }

    // forward, right and up of a key's box; a look-at key's box points at its target
    fn box_axes(key: &PathKey) -> (Vec3, Vec3, Vec3) {
        let to_target = key.target.map(|target| target - key.pose.position);
        let forward = match to_target {
            Some(direction) if direction.length() > MIN_DIRECTION => direction.normalized(),
            _ => key.pose.forward(),
        };
        let cross = forward.cross(WORLD_UP);
        let right = if cross.length() > MIN_DIRECTION {
            cross.normalized()
        } else {
            let (sin_yaw, cos_yaw) = key.pose.yaw.to_radians().sin_cos();
            Vec3::new(-cos_yaw, 0.0, -sin_yaw)
        };
        (forward, right, right.cross(forward).normalized())
    }

    // at right angles to the edge and the box direction; along `right` for an edge on the box direction
    fn bold_offset(edge: Vec3, forward: Vec3, right: Vec3) -> Vec3 {
        let side = edge.cross(forward);
        let side = if side.length() > MIN_DIRECTION * edge.length() { side.normalized() } else { right };
        side * BOLD_OFFSET
    }

    // drops the shape when the buffer is at `max_shapes`
    fn push(&mut self, shape: MarkerShape) {
        if self.shapes.len() < self.config.max_shapes {
            self.shapes.push(shape);
        }
    }
}
