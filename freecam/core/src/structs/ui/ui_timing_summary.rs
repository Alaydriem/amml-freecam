use crate::structs::ui::UiTiming;

// the UI frames measured between two hand-offs to core: sums, maxima and counts, and the newest frame's counters
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UiTimingSummary {
    pub cpu_ms_sum: f32,
    pub cpu_ms_max: f32,
    pub cpu_samples: u32,
    pub gpu_ms_sum: f32,
    pub gpu_ms_max: f32,
    pub gpu_samples: u32,
    pub last: UiTiming,
}

impl UiTimingSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, timing: UiTiming) {
        self.cpu_ms_sum += timing.cpu_ms;
        self.cpu_ms_max = self.cpu_ms_max.max(timing.cpu_ms);
        self.cpu_samples = self.cpu_samples.saturating_add(1);
        if let Some(gpu) = timing.gpu_ms {
            self.gpu_ms_sum += gpu;
            self.gpu_ms_max = self.gpu_ms_max.max(gpu);
            self.gpu_samples = self.gpu_samples.saturating_add(1);
        }
        self.last = timing;
    }

    // true when no frame was added
    pub fn is_empty(&self) -> bool {
        self.cpu_samples == 0
    }
}
