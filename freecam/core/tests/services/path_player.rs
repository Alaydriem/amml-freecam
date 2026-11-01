use freecam_core::services::PathPlayer;
use freecam_core::services::path_player::EASE_RAMP_SECONDS;
use freecam_core::structs::path::PlaybackStep;

struct Fixture;

impl Fixture {
    fn started(length: f32, speed: f32, settle_frames: u32) -> PathPlayer {
        let mut player = PathPlayer::new(EASE_RAMP_SECONDS);
        player.start(length, speed, settle_frames, 0.0);
        player
    }

    fn run(player: &mut PathPlayer, dt: f64) -> Vec<(f64, PlaybackStep)> {
        let mut elapsed = 0.0;
        let mut steps = Vec::new();
        for _ in 0..100_000 {
            elapsed += dt;
            let step = player.step(dt, false);
            steps.push((elapsed, step));
            if step.finished {
                break;
            }
        }
        steps
    }

    fn assert_steady(dt: f64) {
        let mut player = Self::started(100.0, 6.0, 0);
        let steps = Self::run(&mut player, dt);
        let mut previous = 0.0_f32;
        let mut checked = 0;
        for (elapsed, step) in &steps {
            let speed = f64::from(step.distance - previous) / dt;
            if *elapsed >= 1.1 && *elapsed <= 15.5 {
                assert!((speed - 6.0).abs() <= 0.12, "speed {speed} at {elapsed}");
                checked += 1;
            }
            previous = step.distance;
        }
        assert!(checked > 100);
    }

    fn finish_time(dts: &[f64]) -> f64 {
        let mut player = Self::started(100.0, 6.0, 0);
        let mut elapsed = 0.0;
        for index in 0..100_000 {
            let dt = dts[index % dts.len()];
            elapsed += dt;
            if player.step(dt, false).finished {
                return elapsed;
            }
        }
        elapsed
    }
}

#[test]
fn settles_before_moving() {
    let mut player = Fixture::started(100.0, 6.0, 10);
    for _ in 0..10 {
        let step = player.step(1.0 / 60.0, false);
        assert_eq!(step.distance, 0.0);
        assert!(step.settling);
        assert!(!step.finished);
    }
    let step = player.step(1.0 / 60.0, false);
    assert!(!step.settling);
    assert!(step.distance > 0.0);
}

#[test]
fn paused_frames_do_not_count() {
    let mut player = Fixture::started(100.0, 6.0, 2);
    for _ in 0..5 {
        let step = player.step(1.0 / 60.0, true);
        assert_eq!(step.distance, 0.0);
        assert!(step.settling);
    }
    assert!(player.step(1.0 / 60.0, false).settling);
    assert!(player.step(1.0 / 60.0, false).settling);
    let moved = player.step(1.0 / 60.0, false);
    assert!(!moved.settling);
    assert!(moved.distance > 0.0);
    for _ in 0..30 {
        let held = player.step(1.0 / 60.0, true);
        assert_eq!(held.distance, moved.distance);
        assert!(!held.finished);
    }
    assert!(player.step(1.0 / 60.0, false).distance > moved.distance);
}

#[test]
fn steady_between_ramps() {
    Fixture::assert_steady(1.0 / 60.0);
    Fixture::assert_steady(1.0 / 144.0);
}

#[test]
fn ramps_start_and_end_at_zero_speed() {
    let dt = 1.0 / 60.0;
    let mut player = Fixture::started(100.0, 6.0, 0);
    let steps = Fixture::run(&mut player, dt);
    let first = steps.first().map(|(_, step)| step.distance).unwrap_or(f32::MAX);
    assert!(f64::from(first) / dt < 0.05);
    let count = steps.len();
    let before_last = steps[count - 2].1.distance;
    let last = steps[count - 1].1;
    assert!(last.finished);
    assert!(f64::from(last.distance - before_last) / dt < 0.05);
}

#[test]
fn short_path_shrinks_ramps() {
    let dt = 1.0 / 60.0;
    let mut player = Fixture::started(2.0, 6.0, 0);
    let steps = Fixture::run(&mut player, dt);
    let mut previous = 0.0_f32;
    for (_, step) in &steps {
        assert!(f64::from(step.distance - previous) / dt <= 6.0);
        previous = step.distance;
    }
    let last = steps[steps.len() - 1].1;
    assert!(last.finished);
    assert_eq!(last.distance, 2.0);
}

#[test]
fn same_take_twice() {
    let dts = [0.007, 0.016, 0.007, 0.033];
    let mut runs = Vec::new();
    for _ in 0..2 {
        let mut player = Fixture::started(40.0, 6.0, 3);
        let mut distances = Vec::new();
        for index in 0..2000 {
            distances.push(player.step(dts[index % dts.len()], false).distance.to_bits());
        }
        runs.push(distances);
    }
    assert_eq!(runs[0], runs[1]);
}

#[test]
fn uneven_dt_same_wall_time() {
    let steady = Fixture::finish_time(&[1.0 / 60.0]);
    let jittery = Fixture::finish_time(&[0.007, 0.016, 0.007, 0.033]);
    assert!((steady - jittery).abs() <= 0.033, "steady {steady} jittery {jittery}");
}

#[test]
fn zero_speed_finishes_after_settle() {
    let mut player = Fixture::started(50.0, 0.0, 3);
    let paused = player.step(1.0 / 60.0, true);
    assert!(paused.distance.is_finite());
    assert_eq!(paused.distance, 0.0);
    assert!(paused.settling);
    for _ in 0..3 {
        let step = player.step(1.0 / 60.0, false);
        assert!(step.settling);
        assert!(!step.finished);
        assert_eq!(step.distance, 0.0);
    }
    let step = player.step(1.0 / 60.0, false);
    assert!(step.finished);
    assert_eq!(step.distance, 0.0);
    assert!(!player.is_running());
}

#[test]
fn zero_length_paused_step_is_finite() {
    let mut player = Fixture::started(0.0, 6.0, 0);
    let paused = player.step(1.0 / 60.0, true);
    assert_eq!(paused.distance, 0.0);
    assert!(!paused.finished);
    assert!(player.step(1.0 / 60.0, false).finished);
}

#[test]
fn steps_after_finish_hold_length() {
    let mut player = Fixture::started(2.0, 6.0, 0);
    Fixture::run(&mut player, 1.0 / 60.0);
    for paused in [false, true] {
        let step = player.step(1.0 / 60.0, paused);
        assert_eq!(step.distance, 2.0);
        assert!(step.finished);
    }
    let mut player = Fixture::started(100.0, 6.0, 0);
    let moving = player.step(2.0, false);
    player.stop();
    assert!(!player.is_running());
    let held = player.step(1.0 / 60.0, false);
    assert_eq!(held.distance, moving.distance);
    assert!(held.finished);
}

#[test]
fn down_ramp_mirrors_up_ramp() {
    let total = 100.0 / 6.0 + 1.0;
    let mut player = Fixture::started(100.0, 6.0, 0);
    assert!((f64::from(player.step(1.0, false).distance) - 3.0).abs() <= 1e-4);
    let mut player = Fixture::started(100.0, 6.0, 0);
    assert!((f64::from(player.step(total - 1.0, false).distance) - 97.0).abs() <= 1e-4);

    let dt = 1.0 / 144.0;
    let mut player = Fixture::started(100.0, 6.0, 0);
    let mut elapsed = 0.0;
    let mut previous = 0.0_f32;
    let mut previous_speed = f64::MAX;
    loop {
        elapsed += dt;
        let step = player.step(dt, false);
        let speed = f64::from(step.distance - previous) / dt;
        if elapsed > total - 1.0 + dt && !step.finished {
            assert!(speed <= previous_speed + 1e-3, "speed {speed} rose above {previous_speed} at {elapsed}");
            previous_speed = speed;
        }
        previous = step.distance;
        if step.finished {
            break;
        }
    }
}

#[test]
fn start_from_a_distance_settles_there_then_eases_in_from_rest() {
    let mut player = PathPlayer::new(EASE_RAMP_SECONDS);
    player.start(100.0, 6.0, 2, 40.0);
    for _ in 0..2 {
        let step = player.step(0.1, false);
        assert!(step.settling);
        assert_eq!(step.distance, 40.0);
    }
    let paused = player.step(0.1, true);
    assert_eq!(paused.distance, 40.0);

    let moving = player.step(0.5, false);

    // half the 1 s ramp from rest: 6 * 1 * (0.5^3 - 0.5^4 / 2) = 0.5625 blocks
    assert!(!moving.settling);
    assert!((moving.distance - 40.5625).abs() < 1e-3, "distance {}", moving.distance);
}

#[test]
fn start_from_a_distance_ends_with_the_same_down_ramp() {
    let dt = 1.0 / 60.0;
    let mut player = PathPlayer::new(EASE_RAMP_SECONDS);
    player.start(100.0, 6.0, 0, 40.0);
    let mut distances = Vec::new();
    loop {
        let step = player.step(dt, false);
        distances.push(step.distance);
        if step.finished {
            break;
        }
    }

    // 60 blocks at 6 per second plus one ramp
    let seconds = distances.len() as f64 * dt;
    assert!((seconds - 11.0).abs() < 2.0 * dt, "seconds {seconds}");
    assert_eq!(*distances.last().unwrap(), 100.0);
    let last_step = distances[distances.len() - 2] - distances[distances.len() - 3];
    assert!(last_step < 0.01, "last step {last_step}");
}

#[test]
fn stop_during_the_settle_holds_the_start_distance() {
    let mut player = PathPlayer::new(EASE_RAMP_SECONDS);
    player.start(100.0, 6.0, 3, 40.0);
    player.step(0.1, false);

    player.stop();

    assert_eq!(player.step(0.1, false).distance, 40.0);
}

#[test]
fn seconds_at_inverts_the_playback_mapping_ramps_included() {
    let dt = 1.0 / 60.0;
    let mut player = Fixture::started(30.0, 6.0, 0);
    let mut elapsed = 0.0_f64;
    loop {
        let step = player.step(dt, false);
        elapsed += dt;
        if step.finished {
            break;
        }
        let seconds = PathPlayer::seconds_at(30.0, 6.0, EASE_RAMP_SECONDS, step.distance);
        assert!((f64::from(seconds) - elapsed).abs() < 2e-3, "seconds {seconds} at {elapsed}");
    }
    assert_eq!(PathPlayer::seconds_at(30.0, 6.0, EASE_RAMP_SECONDS, 0.0), 0.0);
    let total = PathPlayer::duration(30.0, 6.0, EASE_RAMP_SECONDS);
    assert!((PathPlayer::seconds_at(30.0, 6.0, EASE_RAMP_SECONDS, 30.0) - total).abs() < 1e-4);
    assert_eq!(PathPlayer::seconds_at(30.0, 0.0, EASE_RAMP_SECONDS, 10.0), 0.0);
}
