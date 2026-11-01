use crate::services::path_store::PATH_CAPACITY;
use crate::structs::ui::ToastKind;

// longest toast file name in bytes
pub const TOAST_FILE_CAPACITY: usize = 64;

// what the UI draws this frame
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiSnapshot {
    pub visible: bool,
    pub cursor_mode: bool,
    // the mod manager can take a screenshot; the bar's Screenshot button takes clicks only then
    pub can_screenshot: bool,
    pub point_count: u32,
    // distance along the path of each point; the first `point_count` entries are used, the rest are 0
    pub point_distances: [f32; PATH_CAPACITY],
    // distance along the path, in blocks
    pub playhead: f32,
    // path length in blocks
    pub length: f32,
    // playback time at the playhead, the same time playback shows there
    pub playhead_seconds: f32,
    // the time a playback of the path takes, 0 when it cannot play
    pub total_seconds: f32,
    // playback speed in blocks per second
    pub speed: f32,
    // always below `point_count`
    pub selection: Option<usize>,
    pub playing: bool,
    // the next ClearPoints removes every point
    pub clear_armed: bool,
    pub toast: ToastKind,
    // UTF-8 file name for a Saved toast; the first `toast_file_len` bytes are used
    pub toast_file: [u8; TOAST_FILE_CAPACITY],
    pub toast_file_len: u8,
}

impl Default for UiSnapshot {
    fn default() -> Self {
        Self {
            visible: false,
            cursor_mode: false,
            can_screenshot: false,
            point_count: 0,
            point_distances: [0.0; PATH_CAPACITY],
            playhead: 0.0,
            length: 0.0,
            playhead_seconds: 0.0,
            total_seconds: 0.0,
            speed: 0.0,
            selection: None,
            playing: false,
            clear_armed: false,
            toast: ToastKind::None,
            toast_file: [0; TOAST_FILE_CAPACITY],
            toast_file_len: 0,
        }
    }
}
