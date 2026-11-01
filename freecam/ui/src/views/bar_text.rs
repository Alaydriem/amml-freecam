use amml_ui::draw::{TextStyle, TypeSize};
use amml_ui::font::FontFace;
use egui::Context;
use freecam_core::structs::UiSnapshot;

use super::PointLabels;
use crate::text::{CachedText, FontWatch};

/// The bar's and the toast's changing text, kept between frames. A read-out is laid out again only when its words
/// or the fonts change, so a frame with the same values and the same language allocates no galley.
pub struct BarText {
    fonts: FontWatch,
    generation: u64,
    pub(crate) timecode: CachedText,
    pub(crate) total: CachedText,
    pub(crate) speed: CachedText,
    pub(crate) count: CachedText,
    pub(crate) tag: CachedText,
    pub(crate) tip: CachedText,
    pub(crate) toast: CachedText,
    pub(crate) labels: PointLabels,
}

impl BarText {
    // the playhead time is the bar's largest read-out; Paper has no type size for it
    const TIMECODE_SIZE: f32 = 18.0;
    // the point numbers under the marks
    const LABEL_SIZE: f32 = 10.5;
    const READ_OUT: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);
    const STRONG: TextStyle = TextStyle::new(TypeSize::Note, FontFace::Semibold);

    pub fn new() -> Self {
        let points = UiSnapshot::default().point_distances.len();
        Self {
            fonts: FontWatch::new(),
            generation: 0,
            timecode: CachedText::new(FontFace::Regular.id(Self::TIMECODE_SIZE)),
            total: CachedText::new(Self::READ_OUT.font_id()),
            speed: CachedText::new(Self::READ_OUT.font_id()),
            count: CachedText::new(Self::STRONG.font_id()),
            tag: CachedText::new(Self::STRONG.font_id()),
            tip: CachedText::new(Self::READ_OUT.font_id()),
            toast: CachedText::new(Self::STRONG.font_id()),
            labels: PointLabels::new(FontFace::Regular.id(Self::LABEL_SIZE), points),
        }
    }

    /// Looks at the fonts once per pass, before the bar or the toast paints.
    pub fn begin(&mut self, ctx: &Context) {
        self.generation = self.fonts.observe(ctx);
    }

    /// Drops every kept galley, for a new egui context.
    pub fn invalidate(&mut self) {
        self.fonts.invalidate();
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

impl Default for BarText {
    fn default() -> Self {
        Self::new()
    }
}
