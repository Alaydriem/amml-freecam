use serde::{Deserialize, Serialize};

use crate::config::{KeyConfig, LookAtConfig, MarkerConfig, MotionConfig, ScreenshotConfig};

// the game's camera system updates on every other frame, so 4 frames give at least one update with no Set
const MIN_EXIT_WAIT_FRAMES: u32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FreecamConfig {
    pub keys: KeyConfig,
    pub motion: MotionConfig,
    pub markers: MarkerConfig,
    pub screenshot: ScreenshotConfig,
    pub look_at: LookAtConfig,
    pub exit_wait_frames: u32,
    pub telemetry: bool,
}

impl Default for FreecamConfig {
    fn default() -> Self {
        Self {
            keys: KeyConfig::default(),
            motion: MotionConfig::default(),
            markers: MarkerConfig::default(),
            screenshot: ScreenshotConfig::default(),
            look_at: LookAtConfig::default(),
            exit_wait_frames: 10,
            telemetry: false,
        }
    }
}

impl FreecamConfig {
    // returns a copy with every value moved into its usable range
    pub fn sanitized(self) -> Self {
        Self {
            keys: self.keys.sanitized(),
            motion: self.motion.sanitized(),
            markers: self.markers.sanitized(),
            screenshot: self.screenshot.sanitized(),
            look_at: self.look_at.sanitized(),
            exit_wait_frames: self.exit_wait_frames.max(MIN_EXIT_WAIT_FRAMES),
            telemetry: self.telemetry,
        }
    }
}
