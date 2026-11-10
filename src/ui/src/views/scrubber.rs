use amml_ui::draw::{Corner, Draw};
use amml_ui::glyph::{Glyph, KeyEased, KeyLook};
use amml_ui::motion::Motion;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, color};
use egui::{Align2, CursorIcon, Rect, Sense, Stroke, Ui, pos2, vec2};
use freecam_core::structs::{UiAction, UiSnapshot};

use super::{BarDraw, BarText, Timecode};
use crate::id::FreecamUiId;

/// The scrubber: point marks on a line and the playhead. A drag or a click on the line scrubs; a click on a mark
/// selects that point.
pub struct Scrubber;

impl Scrubber {
    const TRACK_H: f32 = 4.0;
    // room under the line for the point numbers
    const LABEL_SPACE: f32 = 8.0;
    // a point number closer than this to the last one drawn is left out
    const LABEL_GAP: f32 = 14.0;
    const MARK_HIT: [f32; 2] = [18.0, 24.0];
    // a scrub closer than this to the playhead, in blocks, is not sent
    const SCRUB_EPSILON: f32 = 1e-3;

    /// The vertical centre of the line inside `track`.
    pub fn line_y(track: Rect) -> f32 {
        track.center().y - Self::LABEL_SPACE * 0.5
    }

    /// The x of `distance` along a path of `length` blocks inside `track`.
    pub fn x_for(track: Rect, distance: f32, length: f32) -> f32 {
        let fraction = if length > 0.0 { (distance / length).clamp(0.0, 1.0) } else { 0.0 };
        track.left() + track.width() * fraction
    }

    // the distance under screen x inside `track`, clamped to the path
    fn distance_at(track: Rect, x: f32, length: f32) -> f32 {
        ((x - track.left()) / track.width()).clamp(0.0, 1.0) * length
    }

    /// Paints the scrubber in `track` and pushes a scrub or a point selection.
    pub fn show(ui: &mut Ui, track: Rect, snapshot: &UiSnapshot, text: &mut BarText, actions: &mut Vec<UiAction>) {
        let response = ui
            .interact(track, ui.scope_id().with(FreecamUiId::ScrubTrack), Sense::click_and_drag())
            .on_hover_cursor(CursorIcon::ResizeHorizontal);
        let scrubbing = response.dragged();
        let pointer = response.interact_pointer_pos().filter(|_| scrubbing || response.clicked());
        let target = pointer.map(|pointer| Self::distance_at(track, pointer.x, snapshot.length));
        if let Some(distance) = target.filter(|distance| (distance - snapshot.playhead).abs() > Self::SCRUB_EPSILON)
            && snapshot.length > 0.0
        {
            actions.push(UiAction::Scrub(distance));
        }
        let cy = Self::line_y(track);
        let playhead_x = Self::x_for(track, snapshot.playhead, snapshot.length);
        // the marks paint over the playhead, so a selected point stays visible when the camera sits on it
        Self::line(ui, track, cy, playhead_x);
        Self::playhead(ui, playhead_x, cy, scrubbing);
        Self::marks(ui, track, cy, snapshot, text, actions);
        if scrubbing {
            Self::time_tip(ui, playhead_x, cy - 22.0, snapshot.playhead_seconds, text);
        }
    }

    fn line(ui: &Ui, track: Rect, cy: f32, playhead_x: f32) {
        let painter = ui.painter();
        let line = Rect::from_min_max(
            pos2(track.left(), cy - Self::TRACK_H * 0.5),
            pos2(track.right(), cy + Self::TRACK_H * 0.5),
        );
        Draw::fill(painter, line, Corner::Full, <color::LineStrong as ColorToken>::VALUE.gamma_multiply(0.95));
        let played = Rect::from_min_max(line.min, pos2(playhead_x, line.max.y));
        if played.width() > 0.5 {
            let accent = <color::Accent as ColorToken>::VALUE;
            BarDraw::hgradient(painter, played, accent.gamma_multiply(0.55), accent);
        }
    }

    // each mark is the amml-ui eased key glyph, in the in-world point colours: green for the selected (or, with no
    // selection, the last) point, blue for the rest
    fn marks(
        ui: &mut Ui,
        track: Rect,
        cy: f32,
        snapshot: &UiSnapshot,
        text: &mut BarText,
        actions: &mut Vec<UiAction>,
    ) {
        let generation = text.generation();
        let count = (snapshot.point_count as usize).min(snapshot.point_distances.len());
        let highlighted = snapshot.selection.filter(|index| *index < count).or_else(|| count.checked_sub(1));
        let mut last_label = f32::NEG_INFINITY;
        for (index, distance) in snapshot.point_distances[..count].iter().enumerate() {
            let x = Self::x_for(track, *distance, snapshot.length);
            let id = ui.scope_id().with((FreecamUiId::Mark, index));
            let hit = Rect::from_center_size(pos2(x, cy), vec2(Self::MARK_HIT[0], Self::MARK_HIT[1]));
            let response = ui.interact(hit, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if response.clicked() {
                actions.push(UiAction::SelectPoint(index));
            }
            let hover = Motion::flag(ui.ctx(), id.with(FreecamUiId::MarkHover), response.hovered());
            let selected = snapshot.selection == Some(index);
            let lit = Motion::flag(ui.ctx(), id.with(FreecamUiId::MarkLit), highlighted == Some(index));
            let painter = ui.painter();
            let look = if selected { KeyLook::Selected } else { KeyLook::Rest };
            let fill = <color::PathPoint as ColorToken>::VALUE
                .lerp_to_gamma(<color::PathPointSelected as ColorToken>::VALUE, lit);
            let mark =
                Rect::from_center_size(pos2(x, cy), vec2(KeyEased::SELECTED_HALF, KeyEased::SELECTED_HALF) * 2.0);
            KeyEased { look }.paint(painter, mark, fill);
            if selected || x - last_label >= Self::LABEL_GAP {
                last_label = x;
                let color = if selected {
                    <color::PathPointSelected as ColorToken>::VALUE
                } else {
                    <color::TextFaint as ColorToken>::VALUE.lerp_to_gamma(<color::TextDim as ColorToken>::VALUE, hover)
                };
                let label = text.labels.galley(painter, generation, index);
                BarDraw::place_tinted(painter, label, pos2(x, cy + 11.0), Align2::CENTER_TOP, color);
            }
        }
    }

    fn playhead(ui: &Ui, x: f32, cy: f32, scrubbing: bool) {
        let amount = Motion::flag(ui.ctx(), ui.scope_id().with(FreecamUiId::ScrubActive), scrubbing);
        let painter = ui.painter();
        let playhead = <color::Playhead as ColorToken>::VALUE;
        let reach = 13.0 + amount * 3.0;
        painter.line_segment([pos2(x, cy - reach), pos2(x, cy + reach)], Stroke::new(2.0, playhead));
        if amount > 0.0 {
            BarDraw::glow(painter, pos2(x, cy), 14.0, playhead.gamma_multiply(0.35 * amount));
        }
        let knob = 5.5 + amount * 2.0;
        painter.circle_filled(pos2(x, cy), knob, playhead);
        painter.circle_stroke(pos2(x, cy), knob, Stroke::new(1.5, <color::KnobRing as ColorToken>::VALUE));
    }

    fn time_tip(ui: &Ui, x: f32, bottom: f32, seconds: f32, text: &mut BarText) {
        let painter = ui.painter();
        let galley = text.tip.galley(painter, text.generation(), |out| Timecode::write_short(out, seconds));
        let width = galley.size().x + 16.0;
        let rect = Rect::from_min_size(pos2(x - width * 0.5, bottom - 22.0), vec2(width, 22.0));
        let playhead = <color::Playhead as ColorToken>::VALUE;
        Draw::surface(painter, rect, Corner::Control, playhead, None, Elevation::Floating);
        let on = <color::OnAccent as ColorToken>::VALUE;
        BarDraw::place_tinted(painter, galley, rect.center(), Align2::CENTER_CENTER, on);
    }
}
