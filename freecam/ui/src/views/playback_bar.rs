use amml_ui::draw::{Corner, Draw};
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, color};
use amml_ui::widget::IconButton;
use egui::{Area, Id, Order, Rect, Ui};
use freecam_core::structs::{UiAction, UiSnapshot};

use super::{BarButton, BarControls, BarDraw, BarLayout, BarText, Scrubber};
use crate::id::FreecamUiId;
use crate::message::FreecamMessage;

/// The playback bar: a floating pill at the bottom centre, shown in bar mode while no path plays.
pub struct PlaybackBar;

impl PlaybackBar {
    const DIVIDER_REACH: f32 = 16.0;

    pub fn is_shown(snapshot: &UiSnapshot) -> bool {
        snapshot.cursor_mode && !snapshot.playing
    }

    /// Paints the bar when it is shown and pushes the actions clicked. Returns the bar rect, or [`Rect::NOTHING`] when
    /// it is hidden. `text` keeps the read-outs laid out between frames.
    ///
    /// The bar lives in its own area over exactly the bar rect: egui counts the pointer as over egui only above an
    /// area, never above the root background, and the mod manager hands a click to a layer only while egui wants it
    pub fn show(ui: &mut Ui, snapshot: &UiSnapshot, text: &mut BarText, actions: &mut Vec<UiAction>) -> Rect {
        if !Self::is_shown(snapshot) {
            return Rect::NOTHING;
        }
        let layout = BarLayout::new(ui.ctx().content_rect());
        Area::new(Id::unique(FreecamUiId::PlaybackBar)).order(Order::Middle).fixed_pos(layout.bar.min).show(
            ui.ctx(),
            |ui| {
                ui.set_min_size(layout.bar.size());
                Self::contents(ui, &layout, snapshot, text, actions);
            },
        );
        layout.bar
    }

    fn contents(
        ui: &mut Ui,
        layout: &BarLayout,
        snapshot: &UiSnapshot,
        text: &mut BarText,
        actions: &mut Vec<UiAction>,
    ) {
        Self::pill(ui, layout);
        BarControls::transport(ui, layout, snapshot, text, actions);
        BarControls::speed_chip(ui, layout, snapshot, text);
        Scrubber::show(ui, layout.track, snapshot, text, actions);
        let shot = IconButton::new(Icon::Camera);
        let unavailable =
            (!snapshot.can_screenshot).then(|| DisabledReason::message(FreecamMessage::ScreenshotUnavailable));
        if BarButton::icon(ui, layout.shot, FreecamUiId::Screenshot, &shot, unavailable).clicked() {
            actions.push(UiAction::Screenshot);
        }
        match snapshot.selection.filter(|index| *index < snapshot.point_count as usize) {
            Some(index) => BarControls::selected(ui, layout, index, text, actions),
            None => BarControls::idle(ui, layout, snapshot, text, actions),
        }
    }

    fn pill(ui: &Ui, layout: &BarLayout) {
        let painter = ui.painter();
        let rect = layout.bar;
        let fill = <color::BgPanel as ColorToken>::VALUE.gamma_multiply(0.92);
        Draw::surface(
            painter,
            rect,
            Corner::Panel,
            fill,
            Some(<color::LineOverlay as ColorToken>::VALUE),
            Elevation::Floating,
        );
        let cy = rect.center().y;
        let (top, bottom) = (cy - Self::DIVIDER_REACH, cy + Self::DIVIDER_REACH);
        let line = <color::LineStrong as ColorToken>::VALUE;
        BarDraw::vline(painter, layout.divider_x, top, bottom, line);
        BarDraw::vline(painter, layout.shot_divider_x, top, bottom, line);
    }
}
