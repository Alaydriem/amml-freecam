use egui::{Rect, pos2, vec2};

/// Where every part of the playback bar sits on a screen, in points. Every width is fixed, so no part moves when a
/// number or a label changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarLayout {
    /// The pill.
    pub bar: Rect,
    pub rewind: Rect,
    pub play: Rect,
    /// Left edge of the timecode.
    pub timecode_x: f32,
    pub speed: Rect,
    /// The scrubber area: the line, the marks and their numbers.
    pub track: Rect,
    /// The line between the scrubber and the point controls.
    pub divider_x: f32,
    /// Clear points, shown with no selection.
    pub clear: Rect,
    /// Add point, shown with no selection.
    pub add: Rect,
    /// Left edge of the point count, shown with no selection.
    pub count_x: f32,
    /// "Point N", shown with a selection.
    pub tag: Rect,
    /// Re-place, shown with a selection.
    pub replace: Rect,
    /// Delete, shown with a selection.
    pub delete: Rect,
    pub shot: Rect,
    /// The line left of the screenshot button.
    pub shot_divider_x: f32,
}

impl BarLayout {
    pub const WIDTH: f32 = 1120.0;
    pub const HEIGHT: f32 = 64.0;
    /// Gap between the bar and the bottom of the screen.
    pub const BOTTOM: f32 = 40.0;

    const PAD: f32 = 14.0;
    const BUTTON: f32 = 30.0;
    const PLAY: f32 = 36.0;
    const TIMECODE_W: f32 = 170.0;
    const SPEED_W: f32 = 132.0;
    const SPEED_H: f32 = 24.0;
    const TRACK_GAP: f32 = 24.0;
    // the point controls keep this width, so the track has one length with and without a selection
    const RIGHT_W: f32 = 404.0;
    const SHOT: f32 = 34.0;
    const CONTROL_H: f32 = 28.0;
    const TAG_H: f32 = 26.0;
    const CLEAR_W: f32 = 28.0;
    const ADD_W: f32 = 128.0;
    const TAG_W: f32 = 104.0;
    const REPLACE_W: f32 = 112.0;
    const DELETE_W: f32 = 100.0;

    /// The layout at the bottom centre of `screen`.
    pub fn new(screen: Rect) -> Self {
        let bar = Rect::from_min_size(
            pos2(screen.center().x - Self::WIDTH * 0.5, screen.bottom() - Self::BOTTOM - Self::HEIGHT),
            vec2(Self::WIDTH, Self::HEIGHT),
        );
        let cy = bar.center().y;
        let rewind = Self::square(bar.left() + Self::PAD, cy, Self::BUTTON);
        let play = Self::square(rewind.right() + 6.0, cy, Self::PLAY);
        let timecode_x = play.right() + Self::PAD;
        let speed = Rect::from_min_size(
            pos2(timecode_x + Self::TIMECODE_W, cy - Self::SPEED_H * 0.5),
            vec2(Self::SPEED_W, Self::SPEED_H),
        );
        let divider_x = bar.right() - Self::RIGHT_W;
        let track = Rect::from_min_max(
            pos2(speed.right() + Self::TRACK_GAP, bar.top() + 10.0),
            pos2(divider_x - Self::TRACK_GAP, bar.bottom() - 6.0),
        );
        let shot =
            Rect::from_center_size(pos2(bar.right() - Self::PAD - Self::SHOT * 0.5, cy), vec2(Self::SHOT, Self::SHOT));
        let clear = Self::control(divider_x + Self::PAD, cy, Self::CLEAR_W);
        let add = Self::control(clear.right() + 8.0, cy, Self::ADD_W);
        let tag =
            Rect::from_min_size(pos2(divider_x + Self::PAD, cy - Self::TAG_H * 0.5), vec2(Self::TAG_W, Self::TAG_H));
        let replace = Self::control(tag.right() + 8.0, cy, Self::REPLACE_W);
        let delete = Self::control(replace.right() + 6.0, cy, Self::DELETE_W);
        Self {
            bar,
            rewind,
            play,
            timecode_x,
            speed,
            track,
            divider_x,
            clear,
            add,
            count_x: add.right() + Self::PAD,
            tag,
            replace,
            delete,
            shot,
            shot_divider_x: shot.left() - 10.0,
        }
    }

    fn square(left: f32, cy: f32, size: f32) -> Rect {
        Rect::from_min_size(pos2(left, cy - size * 0.5), vec2(size, size))
    }

    fn control(left: f32, cy: f32, width: f32) -> Rect {
        Rect::from_min_size(pos2(left, cy - Self::CONTROL_H * 0.5), vec2(width, Self::CONTROL_H))
    }
}
