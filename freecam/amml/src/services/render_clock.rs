use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// what the render thread's frame handler tells the game thread: frames drawn, the screen size in physical pixels,
// whether the pointer is over the UI, and whether to time the UI. Every value is a plain Relaxed flag or counter;
// no hand-off is ordered through them
pub struct RenderClock {
    frames: AtomicU64,
    // width in the high 32 bits, height in the low 32 bits, 0 before the first frame
    size: AtomicU64,
    pointer_over_ui: AtomicBool,
    measure: AtomicBool,
}

impl RenderClock {
    pub fn new() -> Self {
        Self {
            frames: AtomicU64::new(0),
            size: AtomicU64::new(0),
            pointer_over_ui: AtomicBool::new(false),
            measure: AtomicBool::new(false),
        }
    }

    // render thread, once per drawn Overlay frame
    pub fn record_frame(&self, width: u32, height: u32, pointer_over_ui: bool) {
        self.frames.fetch_add(1, Ordering::Relaxed);
        self.size.store((u64::from(width) << 32) | u64::from(height), Ordering::Relaxed);
        self.pointer_over_ui.store(pointer_over_ui, Ordering::Relaxed);
    }

    // Overlay frames drawn so far, wrapping at u32
    pub fn frames(&self) -> u32 {
        self.frames.load(Ordering::Relaxed) as u32
    }

    // width over height of the last drawn frame, None before one with a size
    pub fn aspect(&self) -> Option<f32> {
        let size = self.size.load(Ordering::Relaxed);
        let (width, height) = ((size >> 32) as u32, size as u32);
        (width > 0 && height > 0).then(|| width as f32 / height as f32)
    }

    pub fn pointer_over_ui(&self) -> bool {
        self.pointer_over_ui.load(Ordering::Relaxed)
    }

    pub fn set_measure(&self, measure: bool) {
        self.measure.store(measure, Ordering::Relaxed);
    }

    pub fn measure(&self) -> bool {
        self.measure.load(Ordering::Relaxed)
    }
}

impl Default for RenderClock {
    fn default() -> Self {
        Self::new()
    }
}
