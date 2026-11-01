use freecam_core::services::PathCurve;
use freecam_core::services::path_store::PATH_CAPACITY;
use freecam_core::structs::CameraPose;
use freecam_core::structs::math::Vec3;

struct Fixture;

impl Fixture {
    fn pose(x: f32, y: f32, z: f32, yaw: f32, pitch: f32) -> CameraPose {
        CameraPose::new(Vec3::new(x, y, z), yaw, pitch)
    }

    fn built(points: &[CameraPose]) -> PathCurve {
        let mut curve = PathCurve::new();
        curve.rebuild(points);
        curve
    }

    fn sweep(curve: &PathCurve) -> Vec<CameraPose> {
        let steps = 400;
        (0..=steps).map(|step| curve.pose_at(curve.length() * step as f32 / steps as f32)).collect()
    }

    fn assert_finite(pose: &CameraPose) {
        let values = [pose.position.x, pose.position.y, pose.position.z, pose.yaw, pose.pitch];
        assert!(values.iter().all(|value| value.is_finite()), "non-finite pose {pose:?}");
    }
}

#[test]
fn passes_through_every_point() {
    let points = [
        Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(4.0, 2.0, 1.0, 30.0, 10.0),
        Fixture::pose(9.0, -1.0, 5.0, 70.0, -20.0),
        Fixture::pose(11.0, 3.0, 12.0, 100.0, 5.0),
        Fixture::pose(20.0, 0.0, 14.0, 140.0, 0.0),
    ];
    let curve = Fixture::built(&points);
    for (index, point) in points.iter().enumerate() {
        let pose = curve.pose_at(curve.point_distance(index));
        assert!((pose.position - point.position).length() < 1e-3, "point {index}: {pose:?}");
        assert!((pose.yaw - point.yaw).abs() < 0.01, "point {index} yaw {}", pose.yaw);
        assert!((pose.pitch - point.pitch).abs() < 0.01, "point {index} pitch {}", pose.pitch);
    }
}

#[test]
fn two_points_are_a_straight_line() {
    let a = Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0);
    let b = Fixture::pose(10.0, 4.0, -6.0, 0.0, 0.0);
    let curve = Fixture::built(&[a, b]);
    let mid = curve.pose_at(curve.length() * 0.5).position;
    let average = (a.position + b.position) * 0.5;
    assert!((mid - average).length() < 1e-4, "{mid:?} vs {average:?}");
}

#[test]
fn uneven_spacing_has_no_loop() {
    let xs = [0.0, 0.1, 10.0, 10.1, 20.0];
    let points = xs.map(|x| Fixture::pose(x, 0.0, 0.0, 0.0, 0.0));
    let curve = Fixture::built(&points);
    let steps = 4000;
    let mut previous = f32::NEG_INFINITY;
    for step in 0..=steps {
        let x = curve.pose_at(curve.length() * step as f32 / steps as f32).position.x;
        assert!(x >= previous - 1e-4, "x backtracked from {previous} to {x}");
        assert!((-1e-3..=20.0 + 1e-3).contains(&x), "x escaped: {x}");
        previous = x;
    }
}

#[test]
fn uneven_hairpin_stays_in_bounding_box() {
    let points = [
        Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(10.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(10.0, 0.0, 0.2, 0.0, 0.0),
        Fixture::pose(0.0, 0.0, 0.2, 0.0, 0.0),
    ];
    let curve = Fixture::built(&points);
    let mut positions = Vec::new();
    curve.sample_positions(32, &mut positions);
    for pose in Fixture::sweep(&curve) {
        positions.push(pose.position);
    }
    for p in positions {
        assert!(p.x >= -0.25 && p.x <= 10.25, "x escaped: {p:?}");
        assert!(p.z >= -0.25 && p.z <= 0.45, "z escaped: {p:?}");
        assert!(p.y.abs() <= 0.25, "y escaped: {p:?}");
    }
}

#[test]
fn zero_length_first_segment_still_starts_at_point_zero() {
    let a = Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0);
    let a_turned = Fixture::pose(0.0, 0.0, 0.0, 90.0, 0.0);
    let b = Fixture::pose(8.0, 0.0, 0.0, 90.0, 0.0);
    let curve = Fixture::built(&[a, a_turned, b]);
    assert!(curve.pose_at(-5.0).yaw.abs() < 1e-2);
    assert!(curve.pose_at(0.0).yaw.abs() < 1e-2);
}

#[test]
fn zero_length_last_segment_still_ends_at_last_point() {
    let a = Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0);
    let b = Fixture::pose(8.0, 0.0, 0.0, 0.0, 0.0);
    let b_turned = Fixture::pose(8.0, 0.0, 0.0, 90.0, 0.0);
    let curve = Fixture::built(&[a, b, b_turned]);
    assert!((curve.pose_at(curve.length() + 5.0).yaw - 90.0).abs() < 1e-2);
    assert!((curve.pose_at(curve.length()).yaw - 90.0).abs() < 1e-2);
}

#[test]
fn non_finite_distance_never_gives_nan() {
    let a = Fixture::pose(0.0, 0.0, 0.0, 10.0, 0.0);
    let b = Fixture::pose(8.0, 0.0, 0.0, 50.0, 0.0);
    let curve = Fixture::built(&[a, b]);
    let nan = curve.pose_at(f32::NAN);
    Fixture::assert_finite(&nan);
    assert!((nan.position - a.position).length() < 1e-4);
    let low = curve.pose_at(f32::NEG_INFINITY);
    Fixture::assert_finite(&low);
    assert!((low.position - a.position).length() < 1e-4);
    let high = curve.pose_at(f32::INFINITY);
    Fixture::assert_finite(&high);
    assert!((high.position - b.position).length() < 1e-3);
}

#[test]
fn coincident_points_are_safe() {
    let a = Fixture::pose(3.0, 3.0, 3.0, 10.0, 5.0);
    let b = Fixture::pose(9.0, 3.0, 3.0, 10.0, 5.0);
    let curve = Fixture::built(&[a, a, b]);
    assert!(curve.length().is_finite() && curve.length() > 0.0);
    for pose in Fixture::sweep(&curve) {
        Fixture::assert_finite(&pose);
    }
    let end = curve.pose_at(curve.length());
    assert!((end.position - b.position).length() < 1e-3, "{end:?}");
}

#[test]
fn all_coincident_points_return_the_first_pose() {
    let a = Fixture::pose(1.0, 2.0, 3.0, 40.0, 10.0);
    let curve = Fixture::built(&[a, a, a]);
    assert_eq!(curve.length(), 0.0);
    assert_eq!(curve.pose_at(7.0), a);
}

#[test]
fn yaw_takes_short_way() {
    let curve = Fixture::built(&[Fixture::pose(0.0, 0.0, 0.0, 170.0, 0.0), Fixture::pose(10.0, 0.0, 0.0, -170.0, 0.0)]);
    for pose in Fixture::sweep(&curve) {
        assert!(pose.yaw >= 170.0 - 1e-3 && pose.yaw <= 190.0 + 1e-3, "yaw {}", pose.yaw);
    }
    let end = curve.pose_at(curve.length()).yaw;
    assert!(((end + 170.0) / 360.0 - ((end + 170.0) / 360.0).round()).abs() < 1e-3, "end yaw {end}");
}

#[test]
fn pitch_clamped() {
    let curve = Fixture::built(&[Fixture::pose(0.0, 0.0, 0.0, 0.0, 89.0), Fixture::pose(5.0, 0.0, 0.0, 0.0, 89.0)]);
    for pose in Fixture::sweep(&curve) {
        assert!(pose.pitch <= 89.9, "pitch {}", pose.pitch);
    }
    let steep = Fixture::built(&[
        Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(5.0, 0.0, 0.0, 0.0, 89.0),
        Fixture::pose(10.0, 0.0, 0.0, 0.0, 89.0),
        Fixture::pose(15.0, 0.0, 0.0, 0.0, 0.0),
    ]);
    for pose in Fixture::sweep(&steep) {
        assert!(pose.pitch <= 89.9 && pose.pitch >= -89.9, "pitch {}", pose.pitch);
    }
}

#[test]
fn distance_is_clamped() {
    let first = Fixture::pose(0.0, 0.0, 0.0, 10.0, 0.0);
    let last = Fixture::pose(8.0, 1.0, 2.0, 50.0, 5.0);
    let curve = Fixture::built(&[first, Fixture::pose(4.0, 3.0, 1.0, 30.0, 2.0), last]);
    let start = curve.pose_at(-5.0);
    let end = curve.pose_at(curve.length() + 5.0);
    assert!((start.position - first.position).length() < 1e-4 && (start.yaw - first.yaw).abs() < 1e-3);
    assert!((end.position - last.position).length() < 1e-3 && (end.yaw - last.yaw).abs() < 1e-2);
}

#[test]
fn rebuild_does_not_reallocate() {
    let mut curve = PathCurve::new();
    let before = curve.sample_capacity();
    let points: Vec<CameraPose> = (0..PATH_CAPACITY)
        .map(|index| Fixture::pose(index as f32, (index % 7) as f32, 0.0, index as f32, 0.0))
        .collect();
    curve.rebuild(&points);
    assert_eq!(curve.sample_capacity(), before);
    curve.rebuild(&points[..3]);
    assert_eq!(curve.sample_capacity(), before);
}

#[test]
fn segment_at_agrees_with_pose_at() {
    let curve = Fixture::built(&[
        Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(10.0, 2.0, 4.0, 90.0, 10.0),
        Fixture::pose(14.0, 0.0, 20.0, 180.0, -10.0),
    ]);
    let length = curve.length();
    for step in 0..=100 {
        let distance = length * step as f32 / 100.0;
        let (segment, t) = curve.segment_at(distance);
        assert!(segment < 2 && (0.0..=1.0).contains(&t), "segment {segment}, t {t}");
        assert_eq!(curve.position_in_segment(segment, t), curve.position_at(distance));
        assert_eq!(curve.pose_in_segment(segment, t), curve.pose_at(distance));
    }
    assert_eq!(curve.segment_at(-1.0), (0, 0.0));
    assert_eq!(curve.segment_at(length + 1.0), (1, 1.0));
}

#[test]
fn segment_lookup_on_a_short_curve_stays_in_bounds() {
    let empty = Fixture::built(&[]);
    assert_eq!(empty.segment_at(3.0), (0, 0.0));
    assert_eq!(empty.position_in_segment(5, 0.5), Vec3::default());
    let single = Fixture::built(&[Fixture::pose(1.0, 2.0, 3.0, 0.0, 0.0)]);
    assert_eq!(single.position_in_segment(5, 0.5), Vec3::new(1.0, 2.0, 3.0));
    let pair = Fixture::built(&[Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0), Fixture::pose(4.0, 0.0, 0.0, 0.0, 0.0)]);
    assert_eq!(pair.position_in_segment(9, 1.0), Vec3::new(4.0, 0.0, 0.0));
}

#[test]
fn paced_curve_keeps_its_points_and_takes_the_pace_spacing() {
    let points = [
        Fixture::pose(0.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(24.0, 0.0, 0.0, 0.0, 0.0),
        Fixture::pose(30.0, 0.0, 0.0, 0.0, 0.0),
    ];
    let even_pace = [
        Fixture::pose(0.0, 5.0, 0.0, 0.0, 0.0),
        Fixture::pose(10.0, 5.0, 0.0, 0.0, 0.0),
        Fixture::pose(20.0, 5.0, 0.0, 0.0, 0.0),
    ];
    let own = Fixture::built(&points);
    let mut paced = PathCurve::new();
    paced.rebuild_paced(&points, &even_pace);

    for (index, point) in points.iter().enumerate() {
        let (segment, t) = if index == 0 { (0, 0.0) } else { (index - 1, 1.0) };
        assert_eq!(paced.position_in_segment(segment, t), point.position);
    }
    // the knot spacing changes the path between the points
    assert!((paced.position_in_segment(0, 0.5) - own.position_in_segment(0, 0.5)).length() > 0.01);

    let mut same = PathCurve::new();
    same.rebuild_paced(&points, &points[..2]);
    assert_eq!(same.position_in_segment(0, 0.5), own.position_in_segment(0, 0.5));
}
