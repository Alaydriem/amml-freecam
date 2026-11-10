// an axis-aligned box (min x, y, z, max x, y, z) and the segment tests the look ray needs, as the game's AABB does
// them: a point is inside only when it is strictly between the faces, and a segment hits at the first face it
// crosses
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl RayBox {
    // the pick margin of an actor that has no PickComponent
    pub const DEFAULT_MARGIN: f32 = 0.1;

    pub fn from_bounds(bounds: [f32; 6]) -> Self {
        Self { min: [bounds[0], bounds[1], bounds[2]], max: [bounds[3], bounds[4], bounds[5]] }
    }

    // the box of the segment from..to
    pub fn spanning(from: [f32; 3], to: [f32; 3]) -> Self {
        Self {
            min: [from[0].min(to[0]), from[1].min(to[1]), from[2].min(to[2])],
            max: [from[0].max(to[0]), from[1].max(to[1]), from[2].max(to[2])],
        }
    }

    pub fn bounds(&self) -> [f32; 6] {
        [self.min[0], self.min[1], self.min[2], self.max[0], self.max[1], self.max[2]]
    }

    pub fn grown(&self, margin: f32) -> Self {
        Self {
            min: [self.min[0] - margin, self.min[1] - margin, self.min[2] - margin],
            max: [self.max[0] + margin, self.max[1] + margin, self.max[2] + margin],
        }
    }

    pub fn contains(&self, point: [f32; 3]) -> bool {
        (0..3).all(|axis| point[axis] > self.min[axis] && point[axis] < self.max[axis])
    }

    // where the segment from..to first enters the box; None when it misses. A start inside the box is a hit at the
    // start, as the game's hit detection reports it
    pub fn hit(&self, from: [f32; 3], to: [f32; 3]) -> Option<[f32; 3]> {
        if self.contains(from) {
            return Some(from);
        }
        let mut enter = 0.0f32;
        let mut leave = 1.0f32;
        for axis in 0..3 {
            let delta = to[axis] - from[axis];
            if delta == 0.0 {
                if from[axis] < self.min[axis] || from[axis] > self.max[axis] {
                    return None;
                }
                continue;
            }
            let near = (self.min[axis] - from[axis]) / delta;
            let far = (self.max[axis] - from[axis]) / delta;
            let (near, far) = if near <= far { (near, far) } else { (far, near) };
            enter = enter.max(near);
            leave = leave.min(far);
            if enter > leave {
                return None;
            }
        }
        Some([
            from[0] + (to[0] - from[0]) * enter,
            from[1] + (to[1] - from[1]) * enter,
            from[2] + (to[2] - from[2]) * enter,
        ])
    }
}
