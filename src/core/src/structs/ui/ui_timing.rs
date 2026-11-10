// the cost of one UI frame, measured by the mod layer
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UiTiming {
    pub cpu_ms: f32,
    pub gpu_ms: Option<f32>,
    pub vertices: u32,
    pub draws: u32,
    // running total of input events the UI queue evicted
    pub dropped_events: u64,
    // running total of UI frames that were not drawn
    pub skipped_frames: u64,
}
