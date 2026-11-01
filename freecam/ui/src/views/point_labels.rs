use std::fmt::Write;
use std::sync::Arc;

use egui::{Color32, FontId, Galley, Painter};

/// The point numbers under the scrubber marks: one galley per number, kept until the font generation changes.
///
/// Bound: one galley per point a path can hold, fixed at construction. A number past that is laid out each time and
/// not kept. A new font generation drops every galley and keeps the slots.
pub struct PointLabels {
    font: FontId,
    scratch: String,
    galleys: Vec<Option<Arc<Galley>>>,
    generation: u64,
}

impl PointLabels {
    pub fn new(font: FontId, capacity: usize) -> Self {
        Self { font, scratch: String::new(), galleys: vec![None; capacity], generation: 0 }
    }

    /// The galley for the number of point `index` (shown from 1), with a placeholder colour.
    pub fn galley(&mut self, painter: &Painter, generation: u64, index: usize) -> Arc<Galley> {
        if self.generation != generation {
            self.galleys.iter_mut().for_each(|galley| *galley = None);
            self.generation = generation;
        }
        if let Some(Some(galley)) = self.galleys.get(index) {
            return Arc::clone(galley);
        }
        self.scratch.clear();
        if write!(self.scratch, "{}", index + 1).is_err() {
            self.scratch.clear();
        }
        let galley = painter.layout_no_wrap(self.scratch.clone(), self.font.clone(), Color32::PLACEHOLDER);
        if let Some(slot) = self.galleys.get_mut(index) {
            *slot = Some(Arc::clone(&galley));
        }
        galley
    }
}
