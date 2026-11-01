use amml_i18n::text::{Args, Text};
use amml_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::text::UiText;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, color};
use egui::{Align2, Rect, Ui, pos2, vec2};
use freecam_core::structs::UiSnapshot;

use super::BarDraw;
use crate::message::FreecamMessage;

/// The chip at the top left: title, point count and shot length.
pub struct Strip;

impl Strip {
    /// Distance from the screen edges, in points.
    pub const MARGIN: f32 = 24.0;
    pub const HEIGHT: f32 = 30.0;
    const TITLE: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Semibold).tracked();
    const DETAIL: TextStyle = TextStyle::new(TypeSize::Note, FontFace::Regular);

    /// The bottom edge of the strip, in points from the top of the screen.
    pub const fn bottom() -> f32 {
        Self::MARGIN + Self::HEIGHT
    }

    /// Paints the strip and returns its rect.
    pub fn show(ui: &Ui, snapshot: &UiSnapshot) -> Rect {
        let painter = ui.painter();
        let screen = ui.ctx().content_rect();
        let text = <color::Text as ColorToken>::VALUE;
        let title = UiText::resolve(ui.ctx(), &Text::message(FreecamMessage::Title));
        let title = Draw::galley(painter, &title, Self::TITLE, text, TextFit::Line);
        let args = Args::new().float("seconds", f64::from(snapshot.total_seconds), 2);
        let detail = Text::plural(FreecamMessage::PathSummary, u64::from(snapshot.point_count), args);
        let detail = UiText::resolve(ui.ctx(), &detail);
        let detail = Draw::galley(painter, &detail, Self::DETAIL, <color::TextDim as ColorToken>::VALUE, TextFit::Line);
        let width = 14.0 + 8.0 + 8.0 + title.size().x + 12.0 + detail.size().x + 16.0;
        let origin = pos2(screen.left() + Self::MARGIN, screen.top() + Self::MARGIN);
        let rect = Rect::from_min_size(origin, vec2(width, Self::HEIGHT));
        let fill = <color::BgOverlay as ColorToken>::VALUE;
        Draw::surface(
            painter,
            rect,
            Corner::Full,
            fill,
            Some(<color::LineOverlay as ColorToken>::VALUE),
            Elevation::Flat,
        );
        let cy = rect.center().y;
        let dot = pos2(rect.left() + 16.0, cy);
        BarDraw::glow(painter, dot, 6.0, <color::Accent as ColorToken>::VALUE.gamma_multiply(0.5));
        painter.circle_filled(dot, 4.0, <color::AccentHover as ColorToken>::VALUE);
        let title_rect = BarDraw::place_tinted(painter, title, pos2(dot.x + 12.0, cy), Align2::LEFT_CENTER, text);
        let detail_pos = pos2(title_rect.right() + 12.0, cy);
        BarDraw::place_tinted(painter, detail, detail_pos, Align2::LEFT_CENTER, <color::TextDim as ColorToken>::VALUE);
        rect
    }
}
