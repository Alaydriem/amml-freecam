use egui::{Context, Rect, Ui};
use freecam_core::structs::{UiAction, UiLegend, UiSnapshot};

use crate::views::{BarText, Legend, PlaybackBar, ScreenshotToast, Strip};

/// The freecam views in one egui pass that the caller runs, as the mod manager runs a layer: the theme and the fonts
/// are the context's. It paints the strip, the legend, the playback bar and the toast, and pushes what the user clicked.
pub struct FreecamUi {
    bar_text: BarText,
    // the pointer was over a panel, or a widget used it, in the last pass while bar mode was on
    over_panel: bool,
}

impl FreecamUi {
    pub fn new() -> Self {
        Self { bar_text: BarText::new(), over_panel: false }
    }

    /// Paints the views for `snapshot` and pushes the clicked actions into `actions`. Returns the union of the drawn
    /// view rects in points, or [`Rect::NOTHING`] when the snapshot is not visible.
    pub fn show(
        &mut self,
        ctx: &Context,
        snapshot: &UiSnapshot,
        legend: &UiLegend,
        actions: &mut Vec<UiAction>,
    ) -> Rect {
        self.over_panel = false;
        if !snapshot.visible {
            return Rect::NOTHING;
        }
        let bar_text = &mut self.bar_text;
        let (rect, over) = ctx.root_ui(|ui| Self::views(ui, snapshot, legend, bar_text, actions));
        self.over_panel = snapshot.cursor_mode && (over || ctx.egui_is_using_pointer());
        rect
    }

    /// True when the last pass had the pointer over a panel in bar mode: a wheel notch or a right press there belongs
    /// to the UI.
    pub fn pointer_over_panel(&self) -> bool {
        self.over_panel
    }

    /// Drops the text laid out against the old font atlas; call it when the context lost its textures.
    pub fn textures_lost(&mut self) {
        self.bar_text.invalidate();
    }

    // paints every view; returns the union of their rects and whether the pointer is over a panel
    fn views(
        ui: &mut Ui,
        snapshot: &UiSnapshot,
        legend: &UiLegend,
        bar_text: &mut BarText,
        actions: &mut Vec<UiAction>,
    ) -> (Rect, bool) {
        let strip = Strip::show(ui, snapshot);
        let legend_rect = Legend::show(ui, legend, snapshot.cursor_mode);
        bar_text.begin(ui.ctx());
        let bar = PlaybackBar::show(ui, snapshot, bar_text, actions);
        let toast = ScreenshotToast::show(ui, snapshot, bar_text);
        // by position, not by layer: the bar's own area sits above this root Ui
        let pointer = ui.ctx().pointer_hover_pos();
        let over = pointer.is_some_and(|at| [strip, legend_rect, bar].into_iter().any(|rect| rect.contains(at)));
        let drawn = [strip, legend_rect, bar, toast].into_iter().filter(|rect| rect.is_positive());
        (drawn.fold(Rect::NOTHING, |all, rect| all.union(rect)), over)
    }
}

impl Default for FreecamUi {
    fn default() -> Self {
        Self::new()
    }
}
