use freecam_core::services::PathSession;
use freecam_core::services::path_player::EASE_RAMP_SECONDS;
use freecam_core::services::path_store::PATH_CAPACITY;
use freecam_core::structs::math::Vec3;
use freecam_core::structs::{CameraPose, PathKey};

const SETTLE: u32 = 3;
const DT: f64 = 0.05;
const MAX_STEPS: usize = 10_000;

struct Fixture;

impl Fixture {
    fn pose(x: f32) -> CameraPose {
        CameraPose::new(Vec3::new(x, 0.0, 0.0), 0.0, 0.0)
    }

    fn key(x: f32) -> PathKey {
        PathKey::new(Self::pose(x), None)
    }

    // a session holding two points, ten blocks apart
    fn two_points(session: &mut PathSession) {
        session.insert(session.keys().len(), Self::key(0.0));
        session.insert(session.keys().len(), Self::key(10.0));
    }
}

#[test]
fn playback_pitch_stays_inside_the_configured_limit() {
    let mut session = PathSession::new(45.0, SETTLE);
    session.insert(session.keys().len(), PathKey::new(CameraPose::new(Vec3::new(0.0, 0.0, 0.0), 0.0, 80.0), None));
    session.insert(session.keys().len(), PathKey::new(CameraPose::new(Vec3::new(10.0, 0.0, 0.0), 30.0, -80.0), None));
    session.insert(session.keys().len(), PathKey::new(CameraPose::new(Vec3::new(20.0, 5.0, 0.0), 60.0, 80.0), None));
    assert!(session.start(6.0, 0.0));
    let mut steps = 0;
    loop {
        let (pose, step) = session.step(DT, false);
        assert!(pose.pitch.abs() <= 45.0, "pitch {}", pose.pitch);
        steps += 1;
        if step.finished || steps >= MAX_STEPS {
            break;
        }
    }
    assert!(steps < MAX_STEPS);
}

#[test]
fn start_needs_two_points() {
    let mut session = PathSession::new(89.9, SETTLE);
    assert!(!session.start(6.0, 0.0));
    assert!(session.step(DT, false).1.finished);
    session.insert(session.keys().len(), Fixture::key(0.0));
    assert!(!session.start(6.0, 0.0));
    let (pose, step) = session.step(DT, false);
    assert!(step.finished);
    assert_eq!(pose, Fixture::pose(0.0));
    assert_eq!(session.points(), &[Fixture::pose(0.0)]);
}

#[test]
fn start_with_two_points_settles_at_the_first_point() {
    let mut session = PathSession::new(89.9, SETTLE);
    Fixture::two_points(&mut session);
    assert!(session.start(6.0, 0.0));
    for _ in 0..SETTLE {
        let (pose, step) = session.step(DT, false);
        assert!(step.settling);
        assert_eq!(pose, Fixture::pose(0.0));
    }
    assert!(!session.step(DT, false).1.settling);
}

#[test]
fn insert_at_capacity_is_refused() {
    let mut session = PathSession::new(89.9, SETTLE);
    for index in 0..PATH_CAPACITY {
        session.insert(session.keys().len(), Fixture::key(index as f32));
    }
    let length = session.curve().length();
    session.insert(session.keys().len(), Fixture::key(9999.0));
    assert_eq!(session.points().len(), PATH_CAPACITY);
    assert_eq!(session.points()[PATH_CAPACITY - 1], Fixture::pose((PATH_CAPACITY - 1) as f32));
    assert_eq!(session.curve().length(), length);
}

#[test]
fn first_frame_in_another_dimension_keeps_the_path() {
    let mut session = PathSession::new(89.9, SETTLE);
    Fixture::two_points(&mut session);
    session.observe_world(true, 1);
    assert_eq!(session.points().len(), 2);
    session.observe_world(true, 1);
    assert_eq!(session.points().len(), 2);
}

#[test]
fn dimension_change_inside_a_world_clears() {
    let mut session = PathSession::new(89.9, SETTLE);
    session.observe_world(true, 0);
    Fixture::two_points(&mut session);
    session.observe_world(true, 1);
    assert!(session.points().is_empty());
    assert_eq!(session.curve().length(), 0.0);
}

#[test]
fn dimension_change_while_out_of_the_world_does_not_clear() {
    let mut session = PathSession::new(89.9, SETTLE);
    session.observe_world(true, 0);
    session.observe_world(false, 0);
    Fixture::two_points(&mut session);
    session.observe_world(false, 1);
    assert_eq!(session.points().len(), 2);
    session.observe_world(true, 1);
    assert_eq!(session.points().len(), 2);
}

#[test]
fn leaving_clears_and_rejoining_does_not_clear_again() {
    let mut session = PathSession::new(89.9, SETTLE);
    session.observe_world(true, 0);
    Fixture::two_points(&mut session);
    session.observe_world(false, 0);
    assert!(session.points().is_empty());
    assert_eq!(session.curve().length(), 0.0);
    Fixture::two_points(&mut session);
    session.observe_world(true, 0);
    assert_eq!(session.points().len(), 2);
}

#[test]
fn leaving_stops_playback() {
    let mut session = PathSession::new(89.9, SETTLE);
    session.observe_world(true, 0);
    Fixture::two_points(&mut session);
    assert!(session.start(6.0, 0.0));
    session.observe_world(false, 0);
    assert!(session.step(DT, false).1.finished);
}

#[test]
fn delete_selected_removes_that_point() {
    let mut session = PathSession::new(45.0, SETTLE);
    session.insert(session.keys().len(), Fixture::key(0.0));
    session.insert(session.keys().len(), Fixture::key(5.0));
    session.insert(session.keys().len(), Fixture::key(10.0));
    assert!(session.delete_selected(Some(1)));
    assert_eq!(session.points(), &[Fixture::pose(0.0), Fixture::pose(10.0)]);
}

#[test]
fn delete_without_selection_changes_nothing() {
    for count in [0, 1, 3] {
        let mut session = PathSession::new(45.0, SETTLE);
        for index in 0..count {
            session.insert(session.keys().len(), Fixture::key(index as f32));
        }
        assert!(!session.delete_selected(None));
        assert_eq!(session.points().len(), count);
    }
}

#[test]
fn delete_selected_past_the_end_changes_nothing() {
    let mut session = PathSession::new(45.0, SETTLE);
    Fixture::two_points(&mut session);
    assert!(!session.delete_selected(Some(2)));
    assert_eq!(session.points().len(), 2);
}

#[test]
fn clear_points_empties_the_path() {
    let mut session = PathSession::new(45.0, SETTLE);
    Fixture::two_points(&mut session);
    assert!(session.clear_points());
    assert!(session.points().is_empty());
    assert!(!session.clear_points());
}

#[test]
fn shot_seconds_is_length_over_speed_plus_ramp() {
    let mut session = PathSession::new(45.0, SETTLE);
    assert_eq!(session.shot_seconds(5.0), 0.0);
    Fixture::two_points(&mut session);
    assert!((session.shot_seconds(5.0) - (2.0 + EASE_RAMP_SECONDS)).abs() < 1e-3);
    assert_eq!(session.shot_seconds(0.0), 0.0);
}

#[test]
fn shot_seconds_shrinks_the_ramp_on_a_short_path() {
    let mut session = PathSession::new(45.0, SETTLE);
    session.insert(session.keys().len(), Fixture::key(0.0));
    session.insert(session.keys().len(), Fixture::key(1.0));

    assert!((session.shot_seconds(5.0) - 0.4).abs() < 1e-3);
}

#[test]
fn shot_seconds_matches_the_playback_time() {
    let mut session = PathSession::new(45.0, 0);
    session.insert(session.keys().len(), Fixture::key(0.0));
    session.insert(session.keys().len(), Fixture::key(1.0));
    let expected = session.shot_seconds(5.0);

    assert!(session.start(5.0, 0.0));
    let mut elapsed = 0.0;
    loop {
        let (_, step) = session.step(0.01, false);
        if step.finished {
            break;
        }
        elapsed += 0.01;
    }

    assert!((elapsed - expected).abs() < 0.03, "played {elapsed}, shown {expected}");
}

#[test]
fn insert_keeps_target() {
    let mut session = PathSession::new(89.9, SETTLE);
    let target = Vec3::new(3.0, 4.0, 5.0);
    session.insert(session.keys().len(), PathKey::new(Fixture::pose(0.0), Some(target)));
    session.insert(session.keys().len(), Fixture::key(10.0));

    assert_eq!(session.keys()[0].target, Some(target));
    assert_eq!(session.keys()[1].target, None);
    assert_eq!(session.points(), &[Fixture::pose(0.0), Fixture::pose(10.0)]);
}

#[test]
fn curve_is_built_from_poses_not_targets() {
    let mut session = PathSession::new(89.9, SETTLE);
    session.insert(session.keys().len(), PathKey::new(Fixture::pose(0.0), Some(Vec3::new(0.0, 50.0, 0.0))));
    session.insert(session.keys().len(), PathKey::new(Fixture::pose(10.0), Some(Vec3::new(0.0, -50.0, 0.0))));

    assert!((session.curve().length() - 10.0).abs() < 1e-3);
    assert!(session.delete_selected(Some(0)));
    assert_eq!(session.points(), &[Fixture::pose(10.0)]);
    assert_eq!(session.curve().length(), 0.0);
}

#[test]
fn step_ratio_stays_in_range_through_look_at_keys() {
    // the v1 measure: each step over the step before it, judged once the speed is steady
    const FRAME: f64 = 1.0 / 60.0;
    const STEADY: f32 = 0.02;
    const MIN_SPEED: f32 = 0.5;
    const MAX_TURN_DEGREES: f32 = 5.0;
    let mut session = PathSession::new(89.9, 0);
    session.insert(
        session.keys().len(),
        PathKey::new(CameraPose::new(Vec3::new(0.0, 64.0, 0.0), 0.0, 0.0), Some(Vec3::new(5.0, 60.0, 10.0))),
    );
    session.insert(session.keys().len(), PathKey::new(CameraPose::new(Vec3::new(20.0, 64.0, 5.0), -60.0, 10.0), None));
    session.insert(
        session.keys().len(),
        PathKey::new(CameraPose::new(Vec3::new(40.0, 66.0, 0.0), 0.0, 0.0), Some(Vec3::new(45.0, 64.0, -8.0))),
    );
    session.insert(session.keys().len(), PathKey::new(CameraPose::new(Vec3::new(60.0, 64.0, 0.0), 90.0, 0.0), None));
    assert!(session.start(6.0, 0.0));

    let mut poses = Vec::new();
    for _ in 0..MAX_STEPS {
        let (pose, step) = session.step(FRAME, false);
        if step.finished {
            break;
        }
        poses.push(pose);
    }
    assert!(poses.len() < MAX_STEPS);

    let steps: Vec<f32> = poses.windows(2).map(|pair| (pair[1].position - pair[0].position).length()).collect();
    let mut judged = 0;
    for window in steps.windows(3) {
        let (earlier, previous, now) = (window[0], window[1], window[2]);
        let steady = (previous - earlier).abs() < earlier * STEADY && previous / FRAME as f32 >= MIN_SPEED;
        if steady {
            let ratio = now / previous;
            assert!((0.8..=1.2).contains(&ratio), "ratio {ratio}");
            judged += 1;
        }
    }
    assert!(judged > 100, "judged {judged}");
    // the same measure on the aim: each turn over the turn before it, judged once the turn rate is steady
    const MIN_TURN_DEGREES: f32 = 0.05;
    let turns: Vec<f32> = poses
        .windows(2)
        .map(|pair| pair[0].forward().dot(pair[1].forward()).clamp(-1.0, 1.0).acos().to_degrees())
        .collect();
    assert!(
        turns.iter().all(|turn| *turn < MAX_TURN_DEGREES),
        "largest turn {:?}",
        turns.iter().copied().reduce(f32::max)
    );
    let mut judged_turns = 0;
    for window in turns.windows(3) {
        let (earlier, previous, now) = (window[0], window[1], window[2]);
        if (previous - earlier).abs() < earlier * STEADY && previous >= MIN_TURN_DEGREES {
            let ratio = now / previous;
            assert!((0.8..=1.2).contains(&ratio), "turn ratio {ratio}");
            judged_turns += 1;
        }
    }
    assert!(judged_turns > 50, "judged turns {judged_turns}");
}
