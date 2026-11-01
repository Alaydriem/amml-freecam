use freecam_core::services::{BarState, PathCurve};
use freecam_core::structs::CameraPose;
use freecam_core::structs::math::Vec3;

struct Fixture;

impl Fixture {
    // a straight curve through points at these x positions
    fn curve(xs: &[f32]) -> PathCurve {
        let poses: Vec<CameraPose> = xs.iter().map(|x| CameraPose::new(Vec3::new(*x, 0.0, 0.0), 0.0, 0.0)).collect();
        let mut curve = PathCurve::new();
        curve.rebuild(&poses);
        curve
    }
}

#[test]
fn insert_after_last_point_at_or_before_playhead() {
    let mut bar = BarState::new();
    let empty = Fixture::curve(&[]);
    assert_eq!(bar.insert_index(&empty, 0), 0);

    let curve = Fixture::curve(&[0.0, 10.0, 20.0]);
    bar.scrub(15.0, curve.length());
    assert_eq!(bar.insert_index(&curve, 3), 2);

    bar.scrub(curve.point_distance(1), curve.length());
    assert_eq!(bar.insert_index(&curve, 3), 2);

    bar.scrub(curve.length(), curve.length());
    assert_eq!(bar.insert_index(&curve, 3), 3);

    bar.rewind();
    assert_eq!(bar.insert_index(&curve, 3), 1);
}

#[test]
fn insert_before_selection_keeps_the_same_point_selected() {
    let mut bar = BarState::new();
    assert!(bar.select(2, 20.0));
    bar.scrub(5.0, 30.0);

    bar.on_inserted(1, 5.0);

    assert_eq!(bar.selection(), Some(3));
    assert_eq!(bar.playhead(), 5.0);

    bar.on_inserted(4, 25.0);
    assert_eq!(bar.selection(), Some(3));
    assert_eq!(bar.playhead(), 25.0);
}

#[test]
fn select_twice_clears_the_selection_and_keeps_the_playhead() {
    let mut bar = BarState::new();

    assert!(bar.select(1, 10.0));
    assert_eq!(bar.selection(), Some(1));
    assert_eq!(bar.playhead(), 10.0);

    assert!(!bar.select(1, 99.0));
    assert_eq!(bar.selection(), None);
    assert_eq!(bar.playhead(), 10.0);
}

#[test]
fn scrub_clamps_to_the_path_and_keeps_the_selection() {
    let mut bar = BarState::new();
    bar.select(1, 10.0);

    bar.scrub(50.0, 20.0);
    assert_eq!(bar.playhead(), 20.0);
    bar.scrub(-3.0, 20.0);
    assert_eq!(bar.playhead(), 0.0);
    bar.scrub(f32::NAN, 20.0);
    assert_eq!(bar.playhead(), 0.0);

    assert_eq!(bar.selection(), Some(1));
}

#[test]
fn play_from_the_end_starts_at_point_one() {
    let mut bar = BarState::new();
    bar.scrub(7.0, 20.0);
    assert_eq!(bar.play_from(20.0), 7.0);

    bar.scrub(20.0, 20.0);
    assert_eq!(bar.play_from(20.0), 0.0);
}

#[test]
fn clamp_drops_a_selection_past_the_end_and_pulls_the_playhead_in() {
    let mut bar = BarState::new();
    bar.select(3, 30.0);

    bar.clamp(4, 40.0);
    assert_eq!(bar.selection(), Some(3));
    assert_eq!(bar.playhead(), 30.0);

    bar.clamp(3, 12.0);
    assert_eq!(bar.selection(), None);
    assert_eq!(bar.playhead(), 12.0);

    bar.clamp(0, 0.0);
    assert_eq!(bar.selection(), None);
    assert_eq!(bar.playhead(), 0.0);
}

#[test]
fn delete_moves_the_playhead_and_clears_the_selection() {
    let mut bar = BarState::new();
    bar.select(2, 20.0);

    bar.on_deleted(10.0);

    assert_eq!(bar.selection(), None);
    assert_eq!(bar.playhead(), 10.0);
}
