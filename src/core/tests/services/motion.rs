use freecam_core::config::MotionConfig;
use freecam_core::services::CameraMotion;
use freecam_core::structs::{CameraPose, HeldKeys};

const EPS: f32 = 1e-4;

struct Fixture;

impl Fixture {
    fn direct() -> CameraMotion {
        let mut motion = CameraMotion::new(MotionConfig::default());
        motion.reset(CameraPose::default());
        motion.toggle_smooth();
        motion
    }

    fn smooth() -> CameraMotion {
        let mut motion = CameraMotion::new(MotionConfig::default());
        motion.reset(CameraPose::default());
        motion
    }

    fn keys_w() -> HeldKeys {
        HeldKeys { forward: true, ..HeldKeys::default() }
    }
}

#[test]
fn direct_forward_moves_along_plus_z_at_yaw_zero() {
    let mut motion = Fixture::direct();

    let pose = motion.step(0.1, Fixture::keys_w());

    assert!((pose.position.z - 0.6).abs() < EPS);
}

#[test]
fn direct_right_moves_minus_x_at_yaw_zero() {
    let mut motion = Fixture::direct();

    let pose = motion.step(0.1, HeldKeys { right: true, ..HeldKeys::default() });

    assert!((pose.position.x + 0.6).abs() < EPS);
}

#[test]
fn space_moves_up() {
    let mut motion = Fixture::direct();

    let pose = motion.step(0.1, HeldKeys { up: true, ..HeldKeys::default() });

    assert!((pose.position.y - 0.6).abs() < EPS);
}

#[test]
fn fast_multiplies_speed() {
    let mut motion = Fixture::direct();

    let pose = motion.step(0.1, HeldKeys { forward: true, fast: true, ..HeldKeys::default() });

    assert!((pose.position.z - 1.8).abs() < EPS);
}

#[test]
fn smooth_accelerates_without_overshoot() {
    let mut motion = Fixture::smooth();
    let frame_times = [0.007, 0.016, 0.007, 0.033];
    let mut previous = 0.0_f32;

    for i in 0..200 {
        motion.step(frame_times[i % frame_times.len()], Fixture::keys_w());
        let z = motion.velocity().z;
        assert!(z >= previous, "velocity fell at step {i}: {z} < {previous}");
        assert!(z <= 6.0, "velocity overshot at step {i}: {z}");
        previous = z;
    }
}

#[test]
fn smooth_glides_to_stop_without_reversing() {
    let mut motion = Fixture::smooth();
    let dt = 1.0 / 60.0;
    for _ in 0..60 {
        motion.step(dt, Fixture::keys_w());
    }
    let mut previous = motion.velocity().z;

    for i in 0..120 {
        motion.step(dt, HeldKeys::default());
        let z = motion.velocity().z;
        assert!(z <= previous, "velocity rose at step {i}: {z} > {previous}");
        assert!(z >= 0.0, "velocity reversed at step {i}: {z}");
        previous = z;
    }

    assert!(previous < 0.01);
}

#[test]
fn frame_rate_independent() {
    let mut slow = Fixture::smooth();
    let mut fast = Fixture::smooth();

    for _ in 0..60 {
        slow.step(1.0 / 60.0, Fixture::keys_w());
    }
    for _ in 0..144 {
        fast.step(1.0 / 144.0, Fixture::keys_w());
    }

    assert!((slow.pose().position.z - fast.pose().position.z).abs() < 0.05);
}

#[test]
fn huge_dt_is_clamped() {
    let mut motion = Fixture::direct();

    let pose = motion.step(5.0, Fixture::keys_w());

    assert!(pose.position.z <= 6.0 * 0.1 + EPS);
}

#[test]
fn zero_dt_changes_nothing() {
    let mut motion = Fixture::direct();
    motion.step(0.05, Fixture::keys_w());
    let before = motion.pose();

    let after = motion.step(0.0, Fixture::keys_w());

    assert_eq!(before, after);
}

#[test]
fn pitch_is_clamped() {
    let mut motion = Fixture::direct();
    motion.add_look(0.0, 10_000.0);
    assert!((motion.step(0.016, HeldKeys::default()).pitch - 89.9).abs() < EPS);

    motion.add_look(0.0, -100_000.0);
    assert!((motion.step(0.016, HeldKeys::default()).pitch + 89.9).abs() < EPS);
}

#[test]
fn two_full_turns_end_on_the_start_view() {
    let mut motion = Fixture::direct();
    let start = motion.pose();
    motion.add_look(4800.0, 0.0);

    let pose = motion.step(0.016, HeldKeys::default());

    assert!(start.forward().dot(pose.forward()) > 1.0 - 1e-6);
    assert!(pose.yaw.abs() <= CameraMotion::YAW_BOUND_DEGREES);
}

#[test]
fn scroll_clamps_to_range() {
    let mut motion = Fixture::direct();
    motion.scroll(100);
    assert_eq!(motion.speed(), 80.0);

    motion.scroll(-100);
    assert_eq!(motion.speed(), 0.5);

    let mut fresh = Fixture::direct();
    fresh.scroll(1);
    assert!((fresh.speed() - 7.5).abs() < EPS);
}

#[test]
fn toggle_smooth_flips() {
    let mut motion = CameraMotion::new(MotionConfig::default());
    assert!(motion.is_smooth());

    let now = motion.toggle_smooth();

    assert!(!now);
    assert!(!motion.is_smooth());
}

#[test]
fn hold_zeroes_velocity() {
    let mut motion = Fixture::smooth();
    motion.step(0.1, Fixture::keys_w());
    assert!(motion.velocity().z > 0.0);

    motion.hold();

    assert_eq!(motion.velocity().z, 0.0);
}

#[test]
fn one_mouse_count_turns_by_the_sensitivity_after_ten_thousand_turns() {
    let mut motion = Fixture::direct();
    let sensitivity = MotionConfig::default().sensitivity;
    motion.add_look(10_000.0 * 360.0 / sensitivity, 0.0);
    let before = motion.step(0.016, HeldKeys::default());

    motion.add_look(1.0, 0.0);
    let after = motion.step(0.016, HeldKeys::default());

    let turn = before.forward().dot(after.forward()).clamp(-1.0, 1.0).acos().to_degrees();
    assert!((turn - sensitivity).abs() < 1e-3, "turned {turn}");
    assert!(after.yaw.abs() <= 720.0);
}
