use freecam_core::config::KeyConfig;
use freecam_core::structs::FrameOutput;

// what one session frame gives the frame driver: core's output, the new keys after a connect or a settings change,
// whether freecam holds its flight leases and the panel is open (the bind matcher's gate), and the path's point count
#[derive(Debug, Clone)]
pub struct SessionFrame {
    pub output: FrameOutput,
    pub keys: Option<KeyConfig>,
    pub holding: bool,
    pub panel_open: bool,
    pub point_count: usize,
}
