use serde::{Deserialize, Serialize};

const MIN_SPEED_FLOOR: f32 = 0.5;
const PITCH_LIMIT_CEILING: f32 = 89.9;
const DEFAULT_MAX_FRAME_DT: f32 = 0.1;
const MAX_FRAME_DT_CEILING: f32 = 1.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionConfig {
    pub start_speed: f32,
    pub min_speed: f32,
    pub max_speed: f32,
    pub scroll_factor: f32,
    pub fast_multiplier: f32,
    // degrees per mouse count
    pub sensitivity: f32,
    pub move_ease_rate: f32,
    pub look_ease_rate: f32,
    pub pitch_limit: f32,
    pub max_frame_dt: f32,
    pub smooth_at_start: bool,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            start_speed: 6.0,
            min_speed: MIN_SPEED_FLOOR,
            max_speed: 80.0,
            scroll_factor: 1.25,
            fast_multiplier: 3.0,
            sensitivity: 0.15,
            move_ease_rate: 6.0,
            look_ease_rate: 14.0,
            pitch_limit: PITCH_LIMIT_CEILING,
            max_frame_dt: DEFAULT_MAX_FRAME_DT,
            smooth_at_start: true,
        }
    }
}

impl MotionConfig {
    // returns a copy with every value moved into its usable range
    pub fn sanitized(self) -> Self {
        let defaults = Self::default();
        let min_speed = Self::finite_or(self.min_speed, defaults.min_speed).max(MIN_SPEED_FLOOR);
        let max_speed = Self::finite_or(self.max_speed, defaults.max_speed).max(min_speed);
        let start_speed = Self::finite_or(self.start_speed, defaults.start_speed).clamp(min_speed, max_speed);
        let sensitivity = Self::finite_or(self.sensitivity, 0.0);
        let pitch_limit = Self::finite_or(self.pitch_limit, 0.0);
        let max_frame_dt = Self::finite_or(self.max_frame_dt, 0.0);
        Self {
            start_speed,
            min_speed,
            max_speed,
            scroll_factor: Self::finite_or(self.scroll_factor, defaults.scroll_factor).max(0.0),
            fast_multiplier: Self::finite_or(self.fast_multiplier, defaults.fast_multiplier).max(0.0),
            sensitivity: if sensitivity > 0.0 { sensitivity } else { defaults.sensitivity },
            move_ease_rate: Self::finite_or(self.move_ease_rate, defaults.move_ease_rate).max(0.0),
            look_ease_rate: Self::finite_or(self.look_ease_rate, defaults.look_ease_rate).max(0.0),
            pitch_limit: if pitch_limit > 0.0 { pitch_limit.min(PITCH_LIMIT_CEILING) } else { defaults.pitch_limit },
            max_frame_dt: if max_frame_dt > 0.0 && max_frame_dt <= MAX_FRAME_DT_CEILING {
                max_frame_dt
            } else {
                DEFAULT_MAX_FRAME_DT
            },
            smooth_at_start: self.smooth_at_start,
        }
    }

    fn finite_or(value: f32, fallback: f32) -> f32 {
        if value.is_finite() { value } else { fallback }
    }
}
