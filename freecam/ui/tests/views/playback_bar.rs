use amml_i18n::localize::Localizer;
use amml_ui::icon::{Icon, IconShape};
use egui::{Pos2, Rect, pos2, vec2};
use freecam_core::structs::{ToastKind, UiAction, UiSnapshot};
use freecam_ui::views::{BarLayout, ScreenshotToast, Scrubber};

use crate::support::ui_fixture::{FrameResult, UiEvent, UiFixture};

const POINT: [u8; 4] = [0x3D, 0xA5, 0xFF, 0xFF];
const POINT_SELECTED: [u8; 4] = [0x4B, 0xE3, 0x5A, 0xFF];
const PLAYHEAD: [u8; 4] = [0xFF, 0x4D, 0x5E, 0xFF];
const ACCENT_HOVER: [u8; 4] = [0x9C, 0x8F, 0xFF, 0xFF];
const LENGTH: f32 = 40.0;

fn layout() -> BarLayout {
    BarLayout::new(Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0)))
}

fn mark(distance: f32) -> Pos2 {
    let track = layout().track;
    pos2(Scrubber::x_for(track, distance, LENGTH), Scrubber::line_y(track))
}

fn click_at(at: Pos2) -> [UiEvent; 3] {
    [
        UiEvent::PointerMoved { x: at.x, y: at.y },
        UiEvent::PointerButton { x: at.x, y: at.y, button: 0, pressed: true },
        UiEvent::PointerButton { x: at.x, y: at.y, button: 0, pressed: false },
    ]
}

fn bar(selection: Option<usize>) -> UiFixture {
    let mut fixture = UiFixture::new().with_path();
    fixture.snapshot.selection = selection;
    fixture.settle();
    fixture
}

fn click(fixture: &mut UiFixture, at: Pos2) -> Vec<UiAction> {
    fixture.frame(&click_at(at)).actions.clone()
}

// indexes of the vertices of `color` within `radius` points of `at`
fn vertices_near(out: &FrameResult, color: [u8; 4], at: Pos2, radius: f32) -> Vec<usize> {
    let near = |x: f32, y: f32| (x - at.x).abs() <= radius && (y - at.y).abs() <= radius;
    out.vertices.iter().enumerate().filter(|(_, v)| v.rgba == color && near(v.x, v.y)).map(|(i, _)| i).collect()
}

#[test]
fn each_control_emits_its_action() {
    let layout = layout();
    let cases = [
        (None, layout.rewind.center(), UiAction::Rewind),
        (None, layout.play.center(), UiAction::Play),
        (None, layout.shot.center(), UiAction::Screenshot),
        (None, layout.add.center(), UiAction::AddPoint),
        (None, layout.clear.center(), UiAction::ClearPoints),
        (None, mark(10.0), UiAction::SelectPoint(1)),
        (Some(1), layout.replace.center(), UiAction::Replace),
        (Some(1), layout.delete.center(), UiAction::Delete),
        (Some(1), mark(10.0), UiAction::SelectPoint(1)),
    ];
    for (selection, at, expected) in cases {
        let mut fixture = bar(selection);

        let actions = click(&mut fixture, at);

        assert_eq!(actions, vec![expected], "selection {selection:?} click at {at:?}");
    }
}

#[test]
fn click_on_the_line_scrubs_to_that_distance() {
    let mut fixture = bar(None);
    let track = layout().track;
    let at = pos2(track.left() + track.width() * 0.6, Scrubber::line_y(track));

    let actions = click(&mut fixture, at);

    assert_eq!(actions.len(), 1, "{actions:?}");
    let UiAction::Scrub(distance) = actions[0] else { panic!("{actions:?}") };
    assert!((distance - 0.6 * LENGTH).abs() < 0.2, "{distance}");
}

#[test]
fn drag_on_the_line_scrubs_with_the_pointer() {
    let mut fixture = bar(None);
    let track = layout().track;
    let y = Scrubber::line_y(track);
    let start = pos2(track.left() + track.width() * 0.6, y);
    let end = pos2(track.left() + track.width() * 0.8, y);
    fixture.frame(&[
        UiEvent::PointerMoved { x: start.x, y: start.y },
        UiEvent::PointerButton { x: start.x, y: start.y, button: 0, pressed: true },
    ]);

    let actions = fixture.frame(&[UiEvent::PointerMoved { x: end.x, y: end.y }]).actions.clone();
    fixture.frame(&[UiEvent::PointerButton { x: end.x, y: end.y, button: 0, pressed: false }]);

    let Some(UiAction::Scrub(distance)) = actions.last().copied() else { panic!("{actions:?}") };
    assert!((distance - 0.8 * LENGTH).abs() < 0.2, "{distance}");
}

#[test]
fn scrub_on_an_empty_path_sends_nothing() {
    let mut fixture = UiFixture::new().cursor_mode();
    fixture.snapshot.point_count = 0;
    fixture.settle();

    let actions = click(&mut fixture, layout().track.center());

    assert!(actions.is_empty(), "{actions:?}");
}

#[test]
fn clear_points_is_a_two_click_arm() {
    let mut fixture = bar(None);
    let clear = layout().clear;
    let away = layout().bar.left_top() + vec2(4.0, 4.0);

    let first = click(&mut fixture, clear.center());
    fixture.frame(&[UiEvent::PointerMoved { x: away.x, y: away.y }]);
    fixture.settle();
    let unarmed = vertices_near(fixture.frame(&[]), ACCENT_HOVER, clear.center(), 16.0).len();
    fixture.snapshot.clear_armed = true;
    let armed = vertices_near(fixture.frame(&[]), ACCENT_HOVER, clear.center(), 16.0).len();
    let second = click(&mut fixture, clear.center());

    assert_eq!(first, vec![UiAction::ClearPoints]);
    assert_eq!(unarmed, 0);
    assert!(armed > 0);
    assert_eq!(second, vec![UiAction::ClearPoints]);
}

#[test]
fn clear_points_paints_the_amml_ui_clear_icon() {
    let mut fixture = bar(None);
    let IconShape::Font(code) = Icon::Clear.shape() else { panic!("the clear icon is a font glyph") };
    let clear = layout().clear;

    let texts = fixture.texts();

    let painted = texts.iter().any(|(text, rect)| *text == code.to_string() && clear.contains(rect.center()));
    assert!(painted, "no clear icon in {clear:?}: {texts:?}");
}

#[test]
fn clear_points_is_disabled_with_zero_points() {
    let mut fixture = UiFixture::new().cursor_mode();
    fixture.snapshot.point_count = 0;
    fixture.settle();

    let actions = click(&mut fixture, layout().clear.center());

    assert!(actions.is_empty(), "{actions:?}");
}

#[test]
fn marks_placed_by_distance() {
    let mut fixture = bar(None);

    let out = fixture.frame(&[]);

    assert!(!vertices_near(out, POINT, mark(0.0), 7.0).is_empty());
    assert!(!vertices_near(out, POINT, mark(10.0), 7.0).is_empty());
    // the last point is green with no selection
    assert!(!vertices_near(out, POINT_SELECTED, mark(40.0), 7.0).is_empty());
    assert!(vertices_near(out, POINT, mark(40.0), 7.0).is_empty());
    // an even spread would put point 2 at the middle of the track
    assert!(vertices_near(out, POINT, mark(20.0), 7.0).is_empty());
}

#[test]
fn selected_mark_draws_over_playhead() {
    let mut fixture = UiFixture::new().with_path();
    fixture.snapshot.selection = Some(1);
    fixture.snapshot.playhead = 10.0;
    fixture.settle();

    let out = fixture.frame(&[]);

    let playhead = vertices_near(out, PLAYHEAD, mark(10.0), 16.0);
    let selected = vertices_near(out, POINT_SELECTED, mark(10.0), 9.0);
    assert!(!playhead.is_empty());
    assert!(!selected.is_empty());
    assert!(playhead.iter().max() < selected.iter().min());
    // the last point turns blue while another point is selected
    assert!(!vertices_near(out, POINT, mark(40.0), 7.0).is_empty());
}

#[test]
fn delete_and_replace_only_with_selection() {
    let layout = layout();
    // the left part of Re-place lies within egui's interact radius of Add point, which shows here
    let replace = layout.replace.right_center() - vec2(20.0, 0.0);
    for at in [replace, layout.delete.center()] {
        let mut fixture = bar(None);

        let actions = click(&mut fixture, at);

        assert!(actions.is_empty(), "click at {at:?}: {actions:?}");
    }
}

#[test]
fn clear_only_without_selection() {
    let layout = layout();
    for at in [layout.clear.center(), layout.add.center()] {
        let mut fixture = bar(Some(0));

        let actions = click(&mut fixture, at);

        assert!(actions.is_empty(), "click at {at:?}: {actions:?}");
    }
}

#[test]
fn bar_hidden_while_playing() {
    let above_bar = layout().bar.top() - 60.0;
    let mut playing = bar(None);
    playing.snapshot.playing = true;
    let mut flying = bar(None);
    flying.snapshot.cursor_mode = false;
    for mut fixture in [playing, flying] {
        fixture.settle();

        let actions = click(&mut fixture, layout().play.center());
        let lowest = fixture.frame(&[]).vertices.iter().map(|v| v.y).fold(0.0_f32, f32::max);

        assert!(actions.is_empty(), "{actions:?}");
        assert!(lowest < above_bar, "{lowest}");
    }
}

#[test]
fn hover_on_the_bar_wants_pointer() {
    let mut fixture = bar(None);
    let at = layout().bar.left_top() + vec2(4.0, 32.0);

    let out = fixture.frame(&[UiEvent::PointerMoved { x: at.x, y: at.y }]);

    assert!(out.wants_pointer);
}

#[test]
fn toast_draws_file_name() {
    let name = b"freecam-20261020-101010.png";
    let mut saved = UiSnapshot { toast: ToastKind::Saved, toast_file_len: name.len() as u8, ..UiSnapshot::default() };
    saved.toast_file[..name.len()].copy_from_slice(name);
    let failed = UiSnapshot { toast: ToastKind::Failed, ..saved };
    let mut broken = saved;
    broken.toast_file[0] = 0xFF;
    let english = Localizer::english();

    assert_eq!(
        ScreenshotToast::message(&saved, &english).as_deref(),
        Some("Screenshot saved: freecam-20261020-101010.png")
    );
    assert_eq!(ScreenshotToast::message(&failed, &english).as_deref(), Some("Screenshot failed"));
    assert_eq!(ScreenshotToast::message(&broken, &english).as_deref(), Some("Screenshot saved: "));
    assert_eq!(ScreenshotToast::message(&UiSnapshot::default(), &english), None);

    let toast_band = |out: &FrameResult| {
        let top = layout().bar.top();
        out.vertices.iter().filter(|v| v.y > top - 50.0 && v.y < top - 10.0).count()
    };
    let mut fixture = UiFixture::new();
    fixture.settle();
    let without = toast_band(fixture.frame(&[]));
    fixture.snapshot.toast = saved.toast;
    fixture.snapshot.toast_file = saved.toast_file;
    fixture.snapshot.toast_file_len = saved.toast_file_len;
    let with_name = toast_band(fixture.frame(&[]));
    fixture.snapshot.toast = ToastKind::Failed;
    let with_failure = toast_band(fixture.frame(&[]));

    assert_eq!(without, 0);
    assert!(with_failure > 0);
    assert!(with_name > with_failure, "{with_name} vs {with_failure}");
}

#[test]
fn bar_layout_matches_poc_widths() {
    let layout = layout();

    assert_eq!(layout.bar, Rect::from_min_max(pos2(400.0, 976.0), pos2(1520.0, 1040.0)));
    assert!(layout.track.width() > 250.0, "{:?}", layout.track);
    assert!(layout.track.right() < layout.divider_x);
    assert!(layout.add.right() < layout.shot_divider_x);
    assert!(layout.delete.right() < layout.shot_divider_x);
    assert!(layout.replace.left() > layout.tag.right());

    let mut fixture = bar(None);
    let out = fixture.frame(&[]);
    let in_bar = out.vertices.iter().filter(|v| v.y > 960.0);
    let (left, right) = in_bar.fold((f32::MAX, f32::MIN), |(l, r), v| (l.min(v.x), r.max(v.x)));
    assert!((370.0..=401.0).contains(&left), "{left}");
    assert!((1519.0..=1550.0).contains(&right), "{right}");
}
