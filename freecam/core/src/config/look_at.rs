use serde::{Deserialize, Serialize};

const MIN_MAX_DISTANCE: f32 = 8.0;
const MAX_MAX_DISTANCE: f32 = 512.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LookAtConfig {
    // the add-point key takes a look-at key when the crosshair ray hits
    pub enabled: bool,
    // ray length in blocks
    pub max_distance: f32,
}

impl Default for LookAtConfig {
    fn default() -> Self {
        Self { enabled: true, max_distance: 256.0 }
    }
}

impl LookAtConfig {
    // returns a copy with every value moved into its usable range, with one warning when any value moved
    pub fn sanitized(self) -> Self {
        let distance = if self.max_distance.is_finite() { self.max_distance } else { Self::default().max_distance };
        let sanitized =
            Self { enabled: self.enabled, max_distance: distance.clamp(MIN_MAX_DISTANCE, MAX_MAX_DISTANCE) };
        if sanitized != self {
            curia::warn!("config look_at values out of range were clamped", { values: format!("{sanitized:?}") });
        }
        sanitized
    }
}
