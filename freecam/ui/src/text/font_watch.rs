use egui::Context;

/// Tells kept galleys when they no longer match the fonts. egui starts its font atlas over when it fills up or when
/// the text options change, and a galley laid out before that points into the old atlas. A galley also belongs to
/// one scale. The generation rises on a scale change, on an atlas that is emptier than at the last look (the atlas
/// only fills between restarts), and on [`Self::invalidate`].
pub struct FontWatch {
    generation: u64,
    pixels_per_point: f32,
    fill_ratio: f32,
}

impl FontWatch {
    pub fn new() -> Self {
        Self { generation: 0, pixels_per_point: 0.0, fill_ratio: 0.0 }
    }

    /// Looks at the fonts once per pass, before a kept galley is used, and returns the generation.
    pub fn observe(&mut self, ctx: &Context) -> u64 {
        let pixels_per_point = ctx.pixels_per_point();
        let fill_ratio = ctx.fonts(|fonts| fonts.font_atlas_fill_ratio());
        if pixels_per_point != self.pixels_per_point || fill_ratio < self.fill_ratio {
            self.generation = self.generation.wrapping_add(1);
        }
        self.pixels_per_point = pixels_per_point;
        self.fill_ratio = fill_ratio;
        self.generation
    }

    /// Drops every kept galley, for a new egui context.
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.fill_ratio = 0.0;
    }
}

impl Default for FontWatch {
    fn default() -> Self {
        Self::new()
    }
}
