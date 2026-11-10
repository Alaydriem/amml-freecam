// one window of frame pacing measurements
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetryReport {
    pub fps: f32,
    pub min_dt: f32,
    pub max_dt: f32,
    // rendered distance per frame over the distance the previous frame's velocity predicts, 1.0 when nothing qualified
    pub step_ratio_min: f32,
    pub step_ratio_max: f32,
    // largest turn of the camera forward vector in one playback frame, degrees
    pub aim_turn_max_deg: f32,
    pub sets_per_second: f32,
    pub camera_updates_per_second: f32,
    // largest angle between the view direction and the game camera direction
    pub max_drift_degrees: f32,
    // frames where freecam expected a rendered position and the mod layer reported none
    pub missed_overrides: u32,
    // UI frame cost over the window, 0 when no sample arrived
    pub ui_cpu_ms_avg: f32,
    pub ui_cpu_ms_max: f32,
    pub ui_gpu_ms_avg: f32,
    pub ui_gpu_ms_max: f32,
    // from the last UI sample
    pub ui_vertices: u32,
    pub ui_draws: u32,
    pub ui_dropped_events: u64,
    pub ui_skipped_frames: u64,
    // marker slots live on the client at the end of the window
    pub marker_live_slots: u32,
    // marker entries sent over the window
    pub marker_entries_sent: u32,
    // the largest single marker batch of the window, in entries and in mod-layer microseconds
    pub marker_max_send_entries: u32,
    pub marker_max_send_us: f32,
    // budget reductions in the last marker build
    pub marker_reductions: u32,
    // the most microseconds one frame spent rebuilding the path curve, the look-at aim and the markers
    pub path_rebuild_us_max: f32,
    // look-at ray requests over the window that got an answer, and that ran out of time
    pub look_at_rays_answered: u32,
    pub look_at_rays_timed_out: u32,
    // milliseconds from a flight key-down to the first frame that rendered the camera somewhere new, over the
    // window's samples; 0 without samples
    pub key_to_move_ms_p50: f32,
    pub key_to_move_ms_p99: f32,
    pub key_to_move_ms_max: f32,
    pub key_to_move_samples: u32,
}
