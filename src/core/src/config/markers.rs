use serde::{Deserialize, Serialize};

// every box and target marker of a full look-at path at line weight 1
const MIN_MAX_SHAPES: usize = 2560;
const MAX_MAX_SHAPES: usize = 8192;
const MIN_MAX_PER_SEND: usize = 50;
const MAX_MAX_PER_SEND: usize = 800;
const MIN_LINE_WEIGHT: u32 = 1;
const MAX_LINE_WEIGHT: u32 = 2;
const MIN_FRUSTUM_DEPTH: f32 = 0.3;
const MAX_FRUSTUM_DEPTH: f32 = 5.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarkerConfig {
    pub enabled: bool,
    // bound of the shape slots, and of every marker buffer
    pub max_shapes: usize,
    // entries in one send, one send per frame
    pub max_per_send: usize,
    // 2 draws each box edge twice for a bolder line
    pub line_weight: u32,
    // blocks from a box's apex to its far face
    pub frustum_depth: f32,
}

impl Default for MarkerConfig {
    fn default() -> Self {
        Self { enabled: true, max_shapes: 4096, max_per_send: 400, line_weight: 2, frustum_depth: 1.2 }
    }
}

impl MarkerConfig {
    // returns a copy with every value moved into its usable range, with one warning when any value moved
    pub fn sanitized(self) -> Self {
        let defaults = Self::default();
        let depth = if self.frustum_depth.is_finite() { self.frustum_depth } else { defaults.frustum_depth };
        let sanitized = Self {
            enabled: self.enabled,
            max_shapes: self.max_shapes.clamp(MIN_MAX_SHAPES, MAX_MAX_SHAPES),
            max_per_send: self.max_per_send.clamp(MIN_MAX_PER_SEND, MAX_MAX_PER_SEND),
            line_weight: self.line_weight.clamp(MIN_LINE_WEIGHT, MAX_LINE_WEIGHT),
            frustum_depth: depth.clamp(MIN_FRUSTUM_DEPTH, MAX_FRUSTUM_DEPTH),
        };
        if sanitized != self {
            curia::warn!("config markers values out of range were clamped", { values: format!("{sanitized:?}") });
        }
        sanitized
    }
}
