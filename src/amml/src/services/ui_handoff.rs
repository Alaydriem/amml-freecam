use freecam_core::config::KeyConfig;
use freecam_core::services::FreecamController;
use freecam_core::structs::{BoundedQueue, UiAction, UiLegend, UiSnapshot, UiTiming, UiTimingSummary};

// what the game thread and the render thread pass each other: the snapshot and the legend one way, the actions and the
// frame timings the other. Each side locks it only to copy in or out, never across its own work. `actions` holds
// ACTION_CAPACITY and evicts the oldest. The legend is copied only when its revision changes, which is a config change
pub struct UiHandoff {
    snapshot: UiSnapshot,
    legend: UiLegend,
    legend_revision: u64,
    actions: BoundedQueue<UiAction>,
    timing: UiTimingSummary,
}

impl UiHandoff {
    pub const ACTION_CAPACITY: usize = 16;

    pub fn new() -> Self {
        Self {
            snapshot: UiSnapshot::default(),
            legend: UiLegend::new(&KeyConfig::default()),
            legend_revision: 0,
            actions: BoundedQueue::new(Self::ACTION_CAPACITY),
            timing: UiTimingSummary::new(),
        }
    }

    // render thread, before the UI frame
    pub fn snapshot(&self) -> UiSnapshot {
        self.snapshot
    }

    // render thread: the legend when its revision is not `seen`
    pub fn legend_since(&self, seen: u64) -> Option<(u64, UiLegend)> {
        (self.legend_revision != seen).then(|| (self.legend_revision, self.legend.clone()))
    }

    // render thread, after the UI frame
    pub fn push_frame(&mut self, actions: &[UiAction], timing: UiTiming) {
        for action in actions {
            self.actions.push(*action);
        }
        self.timing.add(timing);
    }

    // game thread, before the core frame
    pub fn apply_actions(&mut self, controller: &mut FreecamController) {
        while let Some(action) = self.actions.pop() {
            controller.apply_ui_action(action);
        }
    }

    // game thread, after the core frame: stores the new snapshot, the legend when it changed, and hands core every UI
    // frame timed since the last call
    pub fn publish(&mut self, controller: &mut FreecamController) {
        self.snapshot = controller.ui_snapshot();
        if controller.legend_revision() != self.legend_revision {
            self.legend = controller.legend().clone();
            self.legend_revision = controller.legend_revision();
        }
        let timing = std::mem::take(&mut self.timing);
        controller.record_ui_summary(&timing);
    }
}

impl Default for UiHandoff {
    fn default() -> Self {
        Self::new()
    }
}
