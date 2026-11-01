use std::fmt;
use std::sync::Arc;

use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, FontId, Galley, Painter};

/// One line of text that is laid out again only when its words, its width limit or the font generation change. Each
/// frame writes the text into a kept buffer and compares it with the text on show, so a frame whose text did not
/// change lays out no galley. The galley has a placeholder colour; [`crate::views::BarDraw::place_tinted`] paints it in any
/// colour.
pub struct CachedText {
    font: FontId,
    // the text written this frame
    scratch: String,
    // the text the galley holds
    shown: String,
    galley: Option<Arc<Galley>>,
    // the font generation the galley was laid out in
    generation: u64,
    // the width the galley was cut to
    max_width: f32,
}

impl CachedText {
    pub fn new(font: FontId) -> Self {
        let (scratch, shown) = (String::new(), String::new());
        Self { font, scratch, shown, galley: None, generation: 0, max_width: f32::INFINITY }
    }

    /// The galley for the text `write` puts into the buffer, on one line of any width. A write that fails shows no
    /// text.
    pub fn galley(
        &mut self,
        painter: &Painter,
        generation: u64,
        write: impl FnOnce(&mut String) -> fmt::Result,
    ) -> Arc<Galley> {
        self.galley_within(painter, generation, f32::INFINITY, write)
    }

    /// Like [`Self::galley`], cut with an ellipsis to `max_width` points, for a read-out in a fixed part of the bar.
    pub fn galley_within(
        &mut self,
        painter: &Painter,
        generation: u64,
        max_width: f32,
        write: impl FnOnce(&mut String) -> fmt::Result,
    ) -> Arc<Galley> {
        self.scratch.clear();
        if write(&mut self.scratch).is_err() {
            self.scratch.clear();
        }
        match &self.galley {
            Some(galley)
                if self.generation == generation && self.max_width == max_width && self.scratch == self.shown =>
            {
                Arc::clone(galley)
            }
            _ => {
                std::mem::swap(&mut self.scratch, &mut self.shown);
                let mut job = LayoutJob::simple_singleline(self.shown.clone(), self.font.clone(), Color32::PLACEHOLDER);
                job.wrap = TextWrapping::truncate_at_width(max_width);
                let galley = painter.layout_job(job);
                self.galley = Some(Arc::clone(&galley));
                self.generation = generation;
                self.max_width = max_width;
                galley
            }
        }
    }
}
