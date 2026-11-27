use freecam_core::services::OrbitMotion;
use freecam_core::structs::{CameraPose, Vec3};

const EPS: f32 = 1e-4;
const DT: f32 = 0.05;
const SPEED: f32 = 6.0;
const PITCH_LIMIT: f32 = 89.9;

struct Fixture;

impl Fixture {
    fn centre() -> Vec3 {
        Vec3::new(0.0, 64.0, 0.0)
    }

    // radius 10, six blocks above the centre, looking level along +z
    fn camera() -> CameraPose {
        CameraPose::new(Vec3::new(0.0, 70.0, -10.0), 0.0, 0.0)
    }

    fn started() -> OrbitMotion {
        Self::started_at(Self::camera())
    }

    fn started_at(camera: CameraPose) -> OrbitMotion {
        let mut orbit = OrbitMotion::new(PITCH_LIMIT);
        assert!(orbit.start(camera, Self::centre(), SPEED));
        orbit
    }

    fn steps(orbit: &mut OrbitMotion, count: usize) -> Vec<CameraPose> {
        (0..count).map(|_| orbit.step(DT, SPEED)).collect()
    }

    fn horizontal(position: Vec3) -> f32 {
        let offset = position - Self::centre();
        (offset.x * offset.x + offset.z * offset.z).sqrt()
    }

    fn right(pose: CameraPose) -> Vec3 {
        CameraPose::new(Vec3::default(), pose.yaw, 0.0).forward().cross(Vec3::new(0.0, 1.0, 0.0))
    }

    fn travel(a: CameraPose, b: CameraPose) -> f32 {
        (b.position - a.position).length()
    }

    // the angle in degrees the view turns from `a` to `b`, which is what the viewer sees
    fn turn(a: CameraPose, b: CameraPose) -> f32 {
        a.forward().dot(b.forward()).clamp(-1.0, 1.0).acos().to_degrees()
    }

    fn faces_centre(pose: CameraPose) -> f32 {
        pose.forward().dot((Self::centre() - pose.position).normalized())
    }
}

#[test]
fn start_refused_inside_min_radius() {
    let mut orbit = OrbitMotion::new(PITCH_LIMIT);

    let started = orbit.start(CameraPose::new(Vec3::new(0.5, 80.0, 0.5), 0.0, 0.0), Fixture::centre(), SPEED);

    assert!(!started);
}

#[test]
fn start_refused_for_non_finite_position() {
    let mut orbit = OrbitMotion::new(PITCH_LIMIT);

    let started = orbit.start(CameraPose::new(Vec3::new(f32::NAN, 70.0, -10.0), 0.0, 0.0), Fixture::centre(), SPEED);

    assert!(!started);
}

#[test]
fn first_step_does_not_move_or_snap() {
    for start_yaw in [0.0, 37.0, 185.0, -170.0, 720.0 + 15.0] {
        let camera = CameraPose::new(Vec3::new(0.0, 70.0, -10.0), start_yaw, 12.0);
        let mut orbit = Fixture::started_at(camera);

        let pose = orbit.step(0.0, SPEED);

        assert!((pose.position - camera.position).length() < EPS);
        assert!(Fixture::turn(camera, pose) < 0.01, "start yaw {start_yaw}");
    }
}

#[test]
fn keeps_radius_and_height() {
    let mut orbit = Fixture::started();

    for pose in Fixture::steps(&mut orbit, 400) {
        assert!((Fixture::horizontal(pose.position) - 10.0).abs() < 1e-3);
        assert!((pose.position.y - 70.0).abs() < EPS);
    }
}

#[test]
fn faces_the_centre_after_the_ramp() {
    let mut orbit = Fixture::started();

    let pose = Fixture::steps(&mut orbit, 40)[39];

    assert!(Fixture::faces_centre(pose) > 0.9999);
}

#[test]
fn moves_toward_the_camera_right() {
    let mut orbit = Fixture::started();

    let pose = Fixture::steps(&mut orbit, 5)[4];

    assert!((pose.position - Fixture::camera().position).dot(Fixture::right(Fixture::camera())) > 0.0);
}

#[test]
fn moves_toward_the_camera_right_when_facing_away() {
    let away = CameraPose::new(Vec3::new(0.0, 70.0, -10.0), 180.0, 0.0);
    let mut orbit = Fixture::started_at(away);

    let pose = Fixture::steps(&mut orbit, 5)[4];

    assert!((pose.position - away.position).dot(Fixture::right(away)) > 0.0);
}

#[test]
fn speed_ramps_in() {
    let mut orbit = Fixture::started();
    let start = orbit.step(0.0, SPEED);

    let poses = Fixture::steps(&mut orbit, 41);

    assert!(Fixture::travel(start, poses[0]) < 0.1 * SPEED * DT);
    let steady = Fixture::travel(poses[39], poses[40]);
    assert!((steady - SPEED * DT).abs() < 0.01 * SPEED * DT);
}

#[test]
fn view_turns_smoothly_over_a_full_turn() {
    let mut orbit = Fixture::started();
    let mut last = orbit.step(0.0, SPEED);

    for pose in Fixture::steps(&mut orbit, 360) {
        assert!(Fixture::turn(last, pose) <= 5.0);
        last = pose;
    }
}

#[test]
fn ease_in_does_not_snap_when_starting_almost_facing_away() {
    for start_yaw in [180.1, 181.0, 185.0, 190.0, -170.0] {
        let mut orbit = Fixture::started_at(CameraPose::new(Vec3::new(0.0, 70.0, -10.0), start_yaw, 0.0));
        let mut last = orbit.step(0.0, SPEED);

        for pose in Fixture::steps(&mut orbit, 40) {
            let turn = Fixture::turn(last, pose);
            assert!(turn <= 20.0, "start yaw {start_yaw}: turned {turn} in one frame");
            last = pose;
        }
    }
}

#[test]
fn hours_of_orbit_keep_the_step_aim_and_a_small_yaw() {
    let mut orbit = Fixture::started();
    Fixture::steps(&mut orbit, 40);
    for _ in 0..24 {
        orbit.step(3600.0, SPEED);
    }
    let mut last = orbit.step(DT, SPEED);

    for pose in Fixture::steps(&mut orbit, 20) {
        assert!((Fixture::travel(last, pose) - SPEED * DT).abs() < 0.01 * SPEED * DT);
        assert!(Fixture::faces_centre(pose) > 0.9999);
        assert!(pose.yaw.abs() <= 360.0);
        assert!((Fixture::horizontal(pose.position) - 10.0).abs() < 1e-3);
        last = pose;
    }
}

#[test]
fn speed_change_eases_in() {
    let mut orbit = Fixture::started();
    let mut last = Fixture::steps(&mut orbit, 60)[59];
    let faster = SPEED * 1.25;

    let first = orbit.step(DT, faster);

    assert!(Fixture::travel(last, first) < SPEED * DT + 0.25 * (faster - SPEED) * DT);
    last = first;
    for _ in 0..40 {
        last = orbit.step(DT, faster);
    }
    let settled = orbit.step(DT, faster);
    assert!((Fixture::travel(last, settled) - faster * DT).abs() < 0.01 * faster * DT);
}

#[test]
fn stop_eases_out_then_finishes() {
    let mut orbit = Fixture::started();
    let mut last = Fixture::steps(&mut orbit, 40)[39];
    orbit.stop();
    let mut last_travel = f32::MAX;

    for _ in 0..19 {
        let pose = orbit.step(DT, SPEED);
        let travel = Fixture::travel(last, pose);
        assert!(travel <= last_travel + EPS);
        assert!(!orbit.is_finished());
        last_travel = travel;
        last = pose;
    }
    for _ in 0..2 {
        last = orbit.step(DT, SPEED);
    }

    assert!(orbit.is_finished());
    let after = orbit.step(DT, SPEED);
    assert!(Fixture::travel(last, after) < EPS);
    assert!(Fixture::turn(last, after) < 0.01);
}

#[test]
fn restart_after_finish_uses_new_camera() {
    let mut orbit = Fixture::started();
    Fixture::steps(&mut orbit, 10);
    orbit.stop();
    Fixture::steps(&mut orbit, 21);
    assert!(orbit.is_finished());

    assert!(orbit.start(CameraPose::new(Vec3::new(5.0, 66.0, 0.0), 0.0, 0.0), Fixture::centre(), SPEED));

    assert!(!orbit.is_finished());
    for pose in Fixture::steps(&mut orbit, 40) {
        assert!((Fixture::horizontal(pose.position) - 5.0).abs() < 1e-3);
    }
}
