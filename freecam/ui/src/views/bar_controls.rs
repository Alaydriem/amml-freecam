use std::fmt::Write;

use amml_i18n::text::{Args, Text};
use amml_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::glyph::{Glyph, KeyEased, KeyLook};
use amml_ui::icon::{Icon, IconSize};
use amml_ui::motion::Motion;
use amml_ui::state::DisabledReason;
use amml_ui::text::{UiLocale, UiText};
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, color};
use amml_ui::widget::{ButtonKind, IconButton, Leading, Surface, SurfaceLevel, TextButton};
use egui::{Align2, CursorIcon, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use freecam_core::structs::{UiAction, UiSnapshot};

use super::{BarButton, BarDraw, BarLayout, BarText, Timecode};
use crate::id::FreecamUiId;
use crate::message::FreecamMessage;

/// The bar's buttons and read-outs around the scrubber.
pub struct BarControls;

impl BarControls {
    const HINT: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);
    // the speed chip's inner padding, and the least gap between two words in a fixed part
    const CHIP_PAD: f32 = 12.0;
    const CHIP_GAP: f32 = 8.0;
    // the tip label's padding, left and right together, then top and bottom together
    const TIP_PAD: egui::Vec2 = vec2(16.0, 8.0);

    /// Rewind, play and the timecode.
    pub fn transport(
        ui: &mut Ui,
        layout: &BarLayout,
        snapshot: &UiSnapshot,
        text: &mut BarText,
        actions: &mut Vec<UiAction>,
    ) {
        let rewind = IconButton::new(Icon::Previous);
        if BarButton::icon(ui, layout.rewind, FreecamUiId::Rewind, &rewind, None).clicked() {
            actions.push(UiAction::Rewind);
        }
        if Self::play_button(ui, layout.play) {
            actions.push(UiAction::Play);
        }
        Self::timecode(ui, pos2(layout.timecode_x, layout.bar.center().y), snapshot, text);
    }

    /// The read-only speed chip: playback speed follows the scroll wheel.
    pub fn speed_chip(ui: &Ui, layout: &BarLayout, snapshot: &UiSnapshot, text: &mut BarText) {
        let painter = ui.painter();
        let rect = layout.speed;
        let cy = rect.center().y;
        let fill = <color::BgField as ColorToken>::VALUE.gamma_multiply(0.9);
        Draw::surface(
            painter,
            rect,
            Corner::Full,
            fill,
            Some(<color::LineStrong as ColorToken>::VALUE),
            Elevation::Flat,
        );
        // the value is laid out first and keeps its room; the caption is cut to what is left
        let inner = rect.width() - Self::CHIP_PAD * 2.0;
        let localizer = UiLocale::get(ui.ctx());
        let value = text.speed.galley_within(painter, text.generation(), inner, |out| {
            let args = Args::new().float("speed", f64::from(snapshot.speed), 1);
            out.write_str(&Text::format(FreecamMessage::SpeedValue, args).resolve(&localizer))
        });
        let at = pos2(rect.right() - Self::CHIP_PAD, cy);
        let value = BarDraw::place_tinted(painter, value, at, Align2::RIGHT_CENTER, <color::Text as ColorToken>::VALUE);
        let room = value.left() - Self::CHIP_GAP - (rect.left() + Self::CHIP_PAD);
        if room >= Self::CHIP_GAP {
            let caption = Text::message(FreecamMessage::Speed);
            let faint = <color::TextFaint as ColorToken>::VALUE;
            let at = pos2(rect.left() + Self::CHIP_PAD, cy);
            Draw::text(painter, at, Align2::LEFT_CENTER, &caption, BarDraw::CAPS, faint, TextFit::Truncate(room));
        }
    }

    /// Clear points, Add point and the point count, shown with no selection.
    pub fn idle(
        ui: &mut Ui,
        layout: &BarLayout,
        snapshot: &UiSnapshot,
        text: &mut BarText,
        actions: &mut Vec<UiAction>,
    ) {
        let armed = snapshot.clear_armed && snapshot.point_count > 0;
        let disabled = (snapshot.point_count == 0).then(|| DisabledReason::message(FreecamMessage::NothingToClear));
        let clear = BarButton::icon(
            ui,
            layout.clear,
            FreecamUiId::ClearPoints,
            &IconButton::new(Icon::Clear).on(armed),
            disabled,
        );
        if clear.clicked() {
            actions.push(UiAction::ClearPoints);
        }
        let tip_anchor = pos2(layout.clear.center().x, layout.bar.top());
        if armed {
            Self::tip(ui, tip_anchor, &Text::message(FreecamMessage::ClickAgainToClear));
        } else if clear.hovered() && snapshot.point_count > 0 {
            Self::tip(ui, tip_anchor, &Text::message(FreecamMessage::ClearPoints));
        }
        let add = TextButton::new(ButtonKind::Primary, Text::message(FreecamMessage::AddPoint))
            .leading(Leading::Icon(Icon::Add));
        if BarButton::text(ui, layout.add, FreecamUiId::AddPoint, &add, None).clicked() {
            actions.push(UiAction::AddPoint);
        }
        let painter = ui.painter();
        let cy = layout.bar.center().y;
        let localizer = UiLocale::get(ui.ctx());
        let count = text.count.galley(painter, text.generation(), |out| {
            let count = Text::plural(FreecamMessage::PointCount, u64::from(snapshot.point_count), Args::new());
            out.write_str(&count.resolve(&localizer))
        });
        let at = pos2(layout.count_x, cy - 8.0);
        BarDraw::place_tinted(painter, count, at, Align2::LEFT_CENTER, <color::Text as ColorToken>::VALUE);
        // the hint is cut where the bar's next part starts
        let room = layout.shot_divider_x - layout.count_x - 12.0;
        let hint = Text::message(FreecamMessage::AddsAtPlayhead);
        let faint = <color::TextFaint as ColorToken>::VALUE;
        let at = pos2(layout.count_x, cy + 10.0);
        Draw::text(painter, at, Align2::LEFT_CENTER, &hint, Self::HINT, faint, TextFit::Truncate(room));
    }

    /// "Point N", Re-place and Delete, shown with a selection.
    pub fn selected(ui: &mut Ui, layout: &BarLayout, index: usize, text: &mut BarText, actions: &mut Vec<UiAction>) {
        Self::point_tag(ui, layout.tag, index, text);
        let replace = TextButton::new(ButtonKind::Secondary, Text::message(FreecamMessage::Replace))
            .leading(Leading::Icon(Icon::Refresh));
        if BarButton::text(ui, layout.replace, FreecamUiId::Replace, &replace, None).clicked() {
            actions.push(UiAction::Replace);
        }
        let delete = TextButton::new(ButtonKind::Outline, Text::message(FreecamMessage::Delete))
            .leading(Leading::Icon(Icon::Delete));
        if BarButton::text(ui, layout.delete, FreecamUiId::Delete, &delete, None).clicked() {
            actions.push(UiAction::Delete);
        }
    }

    // returns true on the frame it is clicked
    fn play_button(ui: &mut Ui, rect: Rect) -> bool {
        let id = ui.scope_id().with(FreecamUiId::Play);
        let response = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let hover = Motion::flag(ui.ctx(), id.with(FreecamUiId::PlayHover), response.hovered());
        let press = Motion::flag(ui.ctx(), id.with(FreecamUiId::PlayPress), response.is_pointer_button_down_on());
        let painter = ui.painter();
        let center = rect.center();
        let radius = rect.width() * 0.5;
        let accent = <color::Accent as ColorToken>::VALUE;
        let glow = accent.gamma_multiply(0.28 + 0.12 * hover);
        BarDraw::glow(painter, center + egui::Vec2::Y, radius + 2.0 + hover * 2.0, glow);
        let fill = accent
            .lerp_to_gamma(<color::AccentHover as ColorToken>::VALUE, hover)
            .lerp_to_gamma(<color::AccentPress as ColorToken>::VALUE, press);
        painter.circle_filled(center, radius - press * 0.8, fill);
        painter.circle_stroke(center, radius - 0.5, Stroke::new(1.0, <color::LineOverlay as ColorToken>::VALUE));
        // the play triangle reads as centred one point right of the true centre
        let icon = Rect::from_center_size(center + egui::Vec2::X, vec2(radius, radius) * 2.0);
        Icon::Play.paint(painter, icon, IconSize::Regular, <color::OnAccent as ColorToken>::VALUE);
        response.clicked()
    }

    fn timecode(ui: &Ui, left_center: Pos2, snapshot: &UiSnapshot, text: &mut BarText) {
        let painter = ui.painter();
        let generation = text.generation();
        let main =
            text.timecode.galley(painter, generation, |out| Timecode::write_short(out, snapshot.playhead_seconds));
        let total = text.total.galley(painter, generation, |out| {
            out.write_str(" / ")?;
            Timecode::write_short(out, snapshot.total_seconds)
        });
        let main_rect =
            BarDraw::place_tinted(painter, main, left_center, Align2::LEFT_CENTER, <color::Text as ColorToken>::VALUE);
        let total_pos = pos2(main_rect.right() + 1.0, main_rect.bottom() - 3.0);
        BarDraw::place_tinted(painter, total, total_pos, Align2::LEFT_BOTTOM, <color::TextDim as ColorToken>::VALUE);
    }

    fn point_tag(ui: &Ui, rect: Rect, index: usize, text: &mut BarText) {
        let painter = ui.painter();
        let cy = rect.center().y;
        let selected = <color::PathPointSelected as ColorToken>::VALUE;
        Draw::fill(painter, rect, Corner::Control, selected.gamma_multiply(0.12));
        Draw::stroke(painter, rect, Corner::Control, selected.gamma_multiply(0.45));
        let mark =
            Rect::from_center_size(pos2(rect.left() + 18.0, cy), vec2(KeyEased::REST_HALF, KeyEased::REST_HALF) * 2.0);
        KeyEased { look: KeyLook::Rest }.paint(painter, mark, selected);
        let localizer = UiLocale::get(ui.ctx());
        let at = pos2(rect.left() + 30.0, cy);
        let room = rect.right() - Self::CHIP_GAP - at.x;
        let tag = text.tag.galley_within(painter, text.generation(), room, |out| {
            let args = Args::new().uint("number", index as u64 + 1);
            out.write_str(&Text::format(FreecamMessage::PointTag, args).resolve(&localizer))
        });
        BarDraw::place_tinted(painter, tag, at, Align2::LEFT_CENTER, <color::Text as ColorToken>::VALUE);
    }

    // a small label centred above `anchor`, with a gap
    fn tip(ui: &Ui, anchor: Pos2, text: &Text) {
        let painter = ui.painter();
        let words = UiText::resolve(ui.ctx(), text);
        let galley = Draw::galley(painter, &words, Self::HINT, <color::Text as ColorToken>::VALUE, TextFit::Line);
        let size = galley.size() + Self::TIP_PAD;
        let rect = Rect::from_min_size(pos2(anchor.x - size.x * 0.5, anchor.y - 12.0 - size.y), size);
        Surface::paint(painter, rect, SurfaceLevel::Raised, Elevation::Floating);
        BarDraw::place_tinted(
            painter,
            galley,
            rect.center(),
            Align2::CENTER_CENTER,
            <color::Text as ColorToken>::VALUE,
        );
    }
}
