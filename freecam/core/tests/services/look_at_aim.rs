use freecam_core::services::{LookAtAim, PathCurve, PathSession};
use freecam_core::structs::{CameraPose, PathKey, Vec3};

const STEPS: usize = 200;
// largest yaw or pitch change allowed between neighbouring samples
const MAX_STEP_DEGREES: f32 = 3.0;
const PITCH_LIMIT: f32 = 89.9;
const PLAY_SPEED: f32 = 6.0;
const FRAME_SECONDS: f64 = 1.0 / 60.0;
const MAX_FRAMES: usize = 10_000;
// largest forward turn in one playback frame, and largest change of it between neighbouring frames
const MAX_FRAME_TURN_DEGREES: f32 = 3.0;
const MAX_TURN_RATE_STEP_DEGREES: f32 = 1.0;
// the same two bounds while the camera passes a target closer than the blend distance
const MAX_PASS_TURN_DEGREES: f32 = 8.0;
const MAX_PASS_RATE_STEP_DEGREES: f32 = 1.5;
const LIVE_SHORT_FRAME: f64 = 0.0006;
const LIVE_LONG_FRAME: f64 = 0.0161;
const LIVE_SPEEDS: [f32; 2] = [6.0, 20.0];
// share of the v1 rotation's largest turn that the aim may reach at speed 20
const V1_TURN_MARGIN: f32 = 1.25;
// largest angle off a shared orbit target during playback
const MAX_ORBIT_ERROR_DEGREES: f32 = 0.5;

struct Fixture;

impl Fixture {
    fn plain(position: Vec3, yaw: f32, pitch: f32) -> PathKey {
        PathKey::new(CameraPose::new(position, yaw, pitch), None)
    }

    fn look(position: Vec3, target: Vec3) -> PathKey {
        PathKey::new(CameraPose::new(position, 0.0, 0.0), Some(target))
    }

    // a key whose own rotation faces its target, as a key taken with the crosshair on the target
    fn aimed(position: Vec3, target: Vec3) -> PathKey {
        let to = target - position;
        let horizontal = (to.x * to.x + to.z * to.z).sqrt();
        let yaw = (-to.x).atan2(to.z).to_degrees();
        let pitch = (-to.y).atan2(horizontal).to_degrees();
        PathKey::new(CameraPose::new(position, yaw, pitch), Some(target))
    }

    // the forward turn in degrees between neighbouring frames of a playback at 6 blocks per second
    // and 60 fps, with the player's ease in and out
    fn playback_turns(keys: &[PathKey]) -> Vec<f32> {
        let mut session = PathSession::new(PITCH_LIMIT, 0);
        for key in keys {
            session.insert(session.keys().len(), *key);
        }
        assert!(session.start(PLAY_SPEED, 0.0));
        let mut forwards = Vec::new();
        for _ in 0..MAX_FRAMES {
            let (pose, step) = session.step(FRAME_SECONDS, false);
            forwards.push(pose.forward());
            if step.finished {
                break;
            }
        }
        forwards.windows(2).map(|pair| pair[0].dot(pair[1]).clamp(-1.0, 1.0).acos().to_degrees()).collect()
    }

    // the forward turn in degrees between neighbouring frames of a playback at `speed`, with the live
    // frame pattern: two camera setups per drawn frame, 0.6 ms and 16.1 ms apart
    fn live_turns(keys: &[PathKey], speed: f32) -> Vec<f32> {
        let mut session = PathSession::new(PITCH_LIMIT, 0);
        for key in keys {
            session.insert(session.keys().len(), *key);
        }
        assert!(session.start(speed, 0.0));
        let mut forwards = Vec::new();
        for frame in 0..MAX_FRAMES * 4 {
            let dt = if frame % 2 == 0 { LIVE_SHORT_FRAME } else { LIVE_LONG_FRAME };
            let (pose, step) = session.step(dt, false);
            forwards.push(pose.forward());
            if step.finished {
                break;
            }
        }
        forwards.windows(2).map(|pair| pair[0].dot(pair[1]).clamp(-1.0, 1.0).acos().to_degrees()).collect()
    }

    // the largest angle between the camera forward and the direction to `target` over a 60 fps playback
    fn largest_tracking_error(keys: &[PathKey], target: Vec3) -> f32 {
        let mut session = PathSession::new(PITCH_LIMIT, 0);
        for key in keys {
            session.insert(session.keys().len(), *key);
        }
        assert!(session.start(PLAY_SPEED, 0.0));
        let mut largest: f32 = 0.0;
        for _ in 0..MAX_FRAMES {
            let (pose, step) = session.step(FRAME_SECONDS, false);
            let wanted = (target - pose.position).normalized();
            largest = largest.max(pose.forward().dot(wanted).clamp(-1.0, 1.0).acos().to_degrees());
            if step.finished {
                break;
            }
        }
        largest
    }

    // the bound on the largest live-frame turn at `speed`: MAX_PASS_TURN_DEGREES, or at speed 20 a
    // quarter above the v1 rotation's largest turn on the same keys when the keys themselves turn
    // faster than that
    fn live_turn_bound(keys: &[PathKey], speed: f32) -> f32 {
        if speed <= LIVE_SPEEDS[0] {
            return MAX_PASS_TURN_DEGREES;
        }
        let plain: Vec<PathKey> = keys.iter().map(|key| PathKey::new(key.pose, None)).collect();
        MAX_PASS_TURN_DEGREES.max(V1_TURN_MARGIN * Self::largest(&Self::live_turns(&plain, speed)))
    }

    fn largest(turns: &[f32]) -> f32 {
        turns.iter().copied().fold(0.0, f32::max)
    }

    fn largest_rate_step(turns: &[f32]) -> f32 {
        turns.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0, f32::max)
    }

    fn curve(keys: &[PathKey]) -> PathCurve {
        let poses: Vec<CameraPose> = keys.iter().map(|key| key.pose).collect();
        let mut curve = PathCurve::new();
        curve.rebuild(&poses);
        curve
    }

    // walks every segment in order the way playback does; returns (aimed, v1) pairs
    fn sweep(keys: &[PathKey]) -> Vec<(CameraPose, CameraPose)> {
        let curve = Self::curve(keys);
        let mut aim = LookAtAim::new();
        aim.rebuild(keys, &curve);
        let mut out = Vec::new();
        for segment in 0..keys.len() - 1 {
            for step in 0..=STEPS {
                let t = step as f32 / STEPS as f32;
                let fallback = curve.pose_in_segment(segment, t);
                out.push((aim.aim(fallback.position, segment, t, fallback), fallback));
            }
        }
        out
    }

    fn largest_step(poses: &[(CameraPose, CameraPose)]) -> f32 {
        poses
            .windows(2)
            .map(|pair| (pair[1].0.yaw - pair[0].0.yaw).abs().max((pair[1].0.pitch - pair[0].0.pitch).abs()))
            .fold(0.0, f32::max)
    }

    fn assert_faces(pose: CameraPose, target: Vec3) {
        let wanted = (target - pose.position).normalized();
        let error = (pose.forward() - wanted).length();
        assert!(error < 1e-3, "forward {:?}, wanted {wanted:?}", pose.forward());
    }
}

#[test]
fn aim_at_key_faces_its_target() {
    let target = Vec3::new(14.0, 61.0, 6.0);
    let keys = [
        Fixture::plain(Vec3::new(0.0, 64.0, 0.0), 0.0, 0.0),
        Fixture::look(Vec3::new(10.0, 64.0, 0.0), target),
        Fixture::plain(Vec3::new(20.0, 64.0, 0.0), -30.0, 0.0),
    ];
    let curve = Fixture::curve(&keys);
    let mut aim = LookAtAim::new();
    aim.rebuild(&keys, &curve);

    let leaving = aim.aim(keys[1].pose.position, 1, 0.0, curve.pose_in_segment(1, 0.0));
    let arriving = aim.aim(keys[1].pose.position, 0, 1.0, curve.pose_in_segment(0, 1.0));

    Fixture::assert_faces(leaving, target);
    Fixture::assert_faces(arriving, target);
    assert_eq!(leaving.position, keys[1].pose.position);
}

#[test]
fn mixed_segment_blends_without_jump() {
    let target = Vec3::new(15.0, 60.0, 8.0);
    let keys = [
        Fixture::plain(Vec3::new(0.0, 64.0, 0.0), 30.0, 5.0),
        Fixture::look(Vec3::new(10.0, 64.0, 0.0), target),
        Fixture::plain(Vec3::new(20.0, 64.0, 0.0), -40.0, -10.0),
        Fixture::plain(Vec3::new(30.0, 64.0, 4.0), 0.0, 0.0),
    ];

    let poses = Fixture::sweep(&keys);

    let step = Fixture::largest_step(&poses);
    assert!(step < MAX_STEP_DEGREES, "largest step {step}");
    // each plain end of a mixed segment shows its own rotation
    let start = poses[0].0;
    assert!((start.yaw - 30.0).abs() < 1e-3 && (start.pitch - 5.0).abs() < 1e-3, "{start:?}");
    let third = poses[2 * (STEPS + 1) - 1].0;
    assert!((third.yaw + 40.0).abs() < 1e-3 && (third.pitch + 10.0).abs() < 1e-3, "{third:?}");
    Fixture::assert_faces(poses[STEPS + 1].0, target);
}

#[test]
fn synthetic_target_uses_other_end_distance() {
    // the look-at key's target is 5 blocks away, so the plain key aims 5 blocks along its view
    let keys = [
        Fixture::look(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 5.0)),
        Fixture::plain(Vec3::new(10.0, 0.0, 0.0), 0.0, 0.0),
    ];
    let mut aim = LookAtAim::new();
    aim.rebuild(&keys, &Fixture::curve(&keys));

    let from = Vec3::new(5.0, 0.0, 0.0);
    let pose = aim.aim(from, 0, 1.0, CameraPose::new(from, 0.0, 0.0));

    Fixture::assert_faces(pose, Vec3::new(10.0, 0.0, 5.0));
}

#[test]
fn aim_blends_toward_key_rotation_when_target_is_close() {
    // the middle key was taken against a wall: its target is 0.36 blocks out, so the distance to
    // the target curve crosses 0.5 and 2 blocks inside both of its segments
    let near = Vec3::new(0.2, 0.0, 0.3);
    let keys = [
        Fixture::aimed(Vec3::new(0.0, 0.0, -10.0), Vec3::new(5.0, 0.0, -2.0)),
        Fixture::aimed(Vec3::new(0.0, 0.0, 0.0), near),
        Fixture::aimed(Vec3::new(0.0, 0.0, 10.0), Vec3::new(-5.0, 0.0, 18.0)),
    ];
    let curve = Fixture::curve(&keys);
    let mut aim = LookAtAim::new();
    aim.rebuild(&keys, &curve);

    let fallback = curve.pose_in_segment(1, 0.0);
    let at_key = aim.aim(keys[1].pose.position, 1, 0.0, fallback);

    assert_eq!((at_key.yaw, at_key.pitch), (fallback.yaw, fallback.pitch));
    Fixture::assert_faces(at_key, near);
    let poses = Fixture::sweep(&keys);
    assert!(poses.iter().all(|(pose, _)| pose.yaw.is_finite() && pose.pitch.is_finite()));
    // passing the close target turns the camera fast, but with no jump
    let turns = Fixture::playback_turns(&keys);
    let largest = Fixture::largest(&turns);
    assert!(largest < MAX_PASS_TURN_DEGREES, "largest turn {largest}");
    let rate_step = Fixture::largest_rate_step(&turns);
    assert!(rate_step < MAX_PASS_RATE_STEP_DEGREES, "largest turn rate step {rate_step}");
}

#[test]
fn aim_has_no_jump_when_a_close_target_moves_away() {
    // a key taken 0.4 blocks from a wall, then a sideways slide to a far look-at key
    let keys = [
        Fixture::aimed(Vec3::new(0.0, 64.0, 0.0), Vec3::new(0.0, 64.0, 0.4)),
        Fixture::aimed(Vec3::new(10.0, 64.0, 0.0), Vec3::new(20.0, 64.0, 17.3)),
    ];

    let turns = Fixture::playback_turns(&keys);

    let largest = Fixture::largest(&turns);
    assert!(largest < MAX_FRAME_TURN_DEGREES, "largest turn {largest}");
    let rate_step = Fixture::largest_rate_step(&turns);
    assert!(rate_step < MAX_TURN_RATE_STEP_DEGREES, "largest turn rate step {rate_step}");
}

#[test]
fn pitch_stays_inside_limit() {
    for target in [Vec3::new(0.001, 0.0, 0.0), Vec3::new(0.0, 20.0, 0.001)] {
        let keys =
            [Fixture::aimed(Vec3::new(0.0, 10.0, 0.0), target), Fixture::plain(Vec3::new(10.0, 10.0, 0.0), 0.0, 0.0)];
        let mut aim = LookAtAim::new();
        aim.rebuild(&keys, &Fixture::curve(&keys));

        let pose = aim.aim(keys[0].pose.position, 0, 0.0, keys[0].pose);

        assert!(pose.pitch.abs() <= PITCH_LIMIT, "pitch {}", pose.pitch);
        assert!(pose.pitch.abs() > 80.0, "pitch {}", pose.pitch);
    }
}

#[test]
fn no_targets_keeps_v1_rotation() {
    let keys = [
        Fixture::plain(Vec3::new(0.0, 0.0, 0.0), 10.0, 5.0),
        Fixture::plain(Vec3::new(10.0, 2.0, 0.0), 170.0, -20.0),
        Fixture::plain(Vec3::new(20.0, 0.0, 5.0), -170.0, 30.0),
    ];

    let poses = Fixture::sweep(&keys);

    assert!(poses.iter().all(|(aimed, fallback)| aimed == fallback));
}

#[test]
fn aim_has_no_jump_after_two_keys_at_one_position() {
    // a pan in place: two look-at keys at one spot, 90 degrees apart, then a key 20 blocks away that
    // looks the same way as the second
    let spot = Vec3::new(0.0, 64.0, 0.0);
    let keys = [
        Fixture::aimed(spot, Vec3::new(-7.0, 64.0, 7.0)),
        Fixture::aimed(spot, Vec3::new(7.0, 64.0, 7.0)),
        Fixture::aimed(Vec3::new(20.0, 64.0, 0.0), Vec3::new(27.0, 64.0, 7.0)),
    ];

    let turns = Fixture::playback_turns(&keys);

    let largest = Fixture::largest(&turns);
    assert!(largest < MAX_FRAME_TURN_DEGREES, "largest turn {largest}");
}

#[test]
fn aim_has_no_whip_when_a_wall_key_turns_round() {
    // a key 1 block from a wall, then keys that turn round to targets 150 blocks behind
    let keys = [
        Fixture::aimed(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
        Fixture::aimed(Vec3::new(4.0, 0.0, 0.0), Vec3::new(4.0, 0.0, -150.0)),
        Fixture::aimed(Vec3::new(12.0, 0.0, -6.0), Vec3::new(12.0, 0.0, -150.0)),
    ];

    for speed in LIVE_SPEEDS {
        let largest = Fixture::largest(&Fixture::live_turns(&keys, speed));
        let bound = Fixture::live_turn_bound(&keys, speed);
        assert!(largest < bound, "speed {speed}: largest turn {largest}, bound {bound}");
    }
}

#[test]
fn aim_has_no_whip_when_targets_go_ahead_behind_ahead() {
    // targets 100 blocks ahead, behind, ahead, keys 5 blocks apart
    let keys = [
        Fixture::aimed(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 100.0)),
        Fixture::aimed(Vec3::new(5.0, 0.0, 0.0), Vec3::new(5.0, 0.0, -100.0)),
        Fixture::aimed(Vec3::new(10.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 100.0)),
    ];

    for speed in LIVE_SPEEDS {
        let largest = Fixture::largest(&Fixture::live_turns(&keys, speed));
        let bound = Fixture::live_turn_bound(&keys, speed);
        assert!(largest < bound, "speed {speed}: largest turn {largest}, bound {bound}");
    }
}

#[test]
fn orbit_of_one_shared_target_tracks_the_target() {
    // five keys on a half circle of radius 12 round one target, each facing it
    let target = Vec3::new(0.0, 64.0, 0.0);
    let keys: Vec<PathKey> = (0..5)
        .map(|index| {
            let angle = (index as f32 * 45.0).to_radians();
            Fixture::aimed(Vec3::new(12.0 * angle.cos(), 66.0, 12.0 * angle.sin()), target)
        })
        .collect();

    let error = Fixture::largest_tracking_error(&keys, target);

    assert!(error < MAX_ORBIT_ERROR_DEGREES, "largest tracking error {error}");
    for speed in LIVE_SPEEDS {
        let largest = Fixture::largest(&Fixture::live_turns(&keys, speed));
        let bound = Fixture::live_turn_bound(&keys, speed);
        assert!(largest < bound, "speed {speed}: largest turn {largest}, bound {bound}");
    }
}
