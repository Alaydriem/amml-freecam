use freecam_core::config::{FreecamConfig, MarkerConfig};
use freecam_core::services::{MarkerBuilder, PathCurve};
use freecam_core::structs::math::Vec3;
use freecam_core::structs::{CameraPose, MarkerKind, MarkerShape, PathKey};

const BLUE: u32 = 0xFF3D_A5FF;
const GREEN: u32 = 0xFF4B_E35A;
const EPSILON: f32 = 1e-3;
const FOV: f32 = 60.0;
const ASPECT: f32 = 2.0;

struct Fixture;

impl Fixture {
    fn config(line_weight: u32, max_shapes: usize) -> MarkerConfig {
        MarkerConfig { line_weight, max_shapes, ..MarkerConfig::default() }
    }

    fn key(x: f32) -> PathKey {
        PathKey::new(CameraPose::new(Vec3::new(x, 0.0, 0.0), 0.0, 0.0), None)
    }

    fn curve(keys: &[PathKey]) -> PathCurve {
        let poses: Vec<CameraPose> = keys.iter().map(|key| key.pose).collect();
        let mut curve = PathCurve::new();
        curve.rebuild(&poses);
        curve
    }

    fn build(config: MarkerConfig, keys: &[PathKey], fov: f32, aspect: f32) -> (Vec<MarkerShape>, u32) {
        Self::build_selected(config, keys, None, fov, aspect)
    }

    fn build_selected(
        config: MarkerConfig,
        keys: &[PathKey],
        selected: Option<usize>,
        fov: f32,
        aspect: f32,
    ) -> (Vec<MarkerShape>, u32) {
        let curve = Self::curve(keys);
        let mut builder = MarkerBuilder::new(config);
        let shapes = builder.build(keys, &curve, selected, fov, aspect).to_vec();
        (shapes, builder.reductions())
    }

    // a straight path of `count` keys, `spacing` blocks apart along x
    fn line_of_keys(count: usize, spacing: f32) -> Vec<PathKey> {
        (0..count).map(|index| Self::key(index as f32 * spacing)).collect()
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    fn count(shapes: &[MarkerShape], kind: MarkerKind) -> usize {
        shapes.iter().filter(|shape| shape.kind == kind).count()
    }

    // half-width and half-height of the far face of the first box, measured on its corners
    fn far_half_extents(shapes: &[MarkerShape], apex: Vec3) -> (f32, f32, f32) {
        let mut depth: f32 = 0.0;
        let mut half_width: f32 = 0.0;
        let mut half_height: f32 = 0.0;
        for shape in &shapes[0..4] {
            let corner = shape.end - apex;
            depth = depth.max(corner.z);
            half_width = half_width.max(corner.x.abs());
            half_height = half_height.max(corner.y.abs());
        }
        (depth, half_width, half_height)
    }
}

#[test]
fn box_apex_is_the_key_position() {
    let key = PathKey::new(CameraPose::new(Vec3::new(1.0, 2.0, 3.0), 40.0, 10.0), None);
    let (shapes, _) = Fixture::build(Fixture::config(1, 4096), &[key], FOV, ASPECT);

    assert_eq!(shapes.len(), 8);
    for shape in &shapes[0..4] {
        assert_eq!(shape.kind, MarkerKind::Line);
        assert!(Fixture::close(shape.location, key.pose.position), "edge starts at {:?}", shape.location);
    }
}

#[test]
fn far_face_size_follows_fov_aspect_and_depth() {
    let config = Fixture::config(1, 4096);
    let depth = config.frustum_depth;
    let (shapes, _) = Fixture::build(config, &[Fixture::key(0.0)], FOV, ASPECT);

    let (far, half_width, half_height) = Fixture::far_half_extents(&shapes, Vec3::default());
    let expected_height = depth * (FOV / 2.0).to_radians().tan();
    assert!((far - depth).abs() < EPSILON, "depth {far}");
    assert!((half_height - expected_height).abs() < EPSILON, "half-height {half_height}");
    assert!((half_width - expected_height * ASPECT).abs() < EPSILON, "half-width {half_width}");
}

#[test]
fn bad_fov_or_aspect_uses_fallbacks() {
    let config = Fixture::config(1, 4096);
    let depth = config.frustum_depth;
    let fallback_height = depth * 35.0_f32.to_radians().tan();
    for (fov, aspect) in [(0.0, 0.0), (f32::NAN, f32::NAN), (-10.0, -1.0), (200.0, f32::INFINITY)] {
        let (shapes, _) = Fixture::build(config.clone(), &[Fixture::key(0.0)], fov, aspect);

        let (_, half_width, half_height) = Fixture::far_half_extents(&shapes, Vec3::default());
        assert!((half_height - fallback_height).abs() < EPSILON, "fov {fov}: half-height {half_height}");
        assert!(
            (half_width - fallback_height * 16.0 / 9.0).abs() < EPSILON,
            "aspect {aspect}: half-width {half_width}"
        );
    }
}

#[test]
fn look_at_box_points_at_its_target() {
    let key = PathKey::new(CameraPose::new(Vec3::default(), 0.0, 0.0), Some(Vec3::new(10.0, 0.0, 0.0)));
    let (shapes, _) = Fixture::build(Fixture::config(1, 4096), &[key], FOV, ASPECT);

    let mut centre = Vec3::default();
    for shape in &shapes[0..4] {
        centre = centre + shape.end * 0.25;
    }
    let direction = centre.normalized();
    assert!(Fixture::close(direction, Vec3::new(1.0, 0.0, 0.0)), "box points along {direction:?}");
}

#[test]
fn last_point_is_green_others_blue() {
    let keys = Fixture::line_of_keys(3, 10.0);
    let (shapes, _) = Fixture::build(Fixture::config(1, 4096), &keys, FOV, ASPECT);

    assert!(shapes[0..16].iter().all(|shape| shape.color == BLUE));
    assert!(shapes[16..24].iter().all(|shape| shape.color == GREEN));
    assert!(shapes[24..].iter().all(|shape| shape.color == BLUE));
    assert!(shapes.len() > 24);
}

#[test]
fn selected_key_box_is_green_and_last_is_blue() {
    let keys = Fixture::line_of_keys(3, 10.0);
    let (shapes, _) = Fixture::build_selected(Fixture::config(1, 4096), &keys, Some(0), FOV, ASPECT);

    assert!(shapes[0..8].iter().all(|shape| shape.color == GREEN));
    assert!(shapes[8..].iter().all(|shape| shape.color == BLUE));
    assert!(shapes.len() > 24);
}

#[test]
fn selection_past_the_end_keeps_the_last_key_green() {
    let keys = Fixture::line_of_keys(3, 10.0);
    let (shapes, _) = Fixture::build_selected(Fixture::config(1, 4096), &keys, Some(3), FOV, ASPECT);

    assert!(shapes[0..16].iter().all(|shape| shape.color == BLUE));
    assert!(shapes[16..24].iter().all(|shape| shape.color == GREEN));
}

#[test]
fn single_point_is_green() {
    let (shapes, _) = Fixture::build(MarkerConfig::default(), &[Fixture::key(0.0)], FOV, ASPECT);

    assert_eq!(shapes.len(), 16);
    assert!(shapes.iter().all(|shape| shape.color == GREEN));
}

#[test]
fn weight_two_draws_box_edges_twice() {
    let (bold, _) = Fixture::build(Fixture::config(2, 4096), &[Fixture::key(0.0)], FOV, ASPECT);
    let (thin, _) = Fixture::build(Fixture::config(1, 4096), &[Fixture::key(0.0)], FOV, ASPECT);

    assert_eq!(Fixture::count(&bold, MarkerKind::Line), 16);
    assert_eq!(Fixture::count(&thin, MarkerKind::Line), 8);
    for edge in 0..8 {
        let offset = (bold[edge + 8].location - bold[edge].location).length();
        let end_offset = (bold[edge + 8].end - bold[edge].end).length();
        assert!((offset - 0.03).abs() < EPSILON, "edge {edge} copy offset {offset}");
        assert!((end_offset - 0.03).abs() < EPSILON, "edge {edge} copy end offset {end_offset}");
    }
}

#[test]
fn one_arrow_per_segment_at_arc_middle() {
    let keys = [Fixture::key(0.0), Fixture::key(10.0), Fixture::key(30.0)];
    let curve = Fixture::curve(&keys);
    let mut builder = MarkerBuilder::new(Fixture::config(1, 4096));
    let shapes = builder.build(&keys, &curve, None, FOV, ASPECT).to_vec();

    let arrows: Vec<&MarkerShape> = shapes.iter().filter(|shape| shape.kind == MarkerKind::Arrow).collect();
    assert_eq!(arrows.len(), 2);
    for (segment, arrow) in arrows.iter().enumerate() {
        let middle = (curve.point_distance(segment) + curve.point_distance(segment + 1)) / 2.0;
        let centre = (arrow.location + arrow.end) * 0.5;
        assert!(Fixture::close(centre, curve.position_at(middle)), "arrow {segment} centred at {centre:?}");
        let direction = (arrow.end - arrow.location).normalized();
        assert!(Fixture::close(direction, Vec3::new(1.0, 0.0, 0.0)), "arrow {segment} points {direction:?}");
        assert_eq!(arrow.color, BLUE);
    }
    let first_arrow = shapes.iter().position(|shape| shape.kind == MarkerKind::Arrow);
    assert_eq!(first_arrow, Some(shapes.len() - 2));
}

#[test]
fn target_marker_is_a_box_in_key_colour_with_no_line_to_the_key() {
    let target = Vec3::new(0.0, 0.0, 200.0);
    let keys = [
        PathKey::new(CameraPose::new(Vec3::default(), 0.0, 0.0), Some(target)),
        PathKey::new(CameraPose::new(Vec3::new(10.0, 0.0, 0.0), 0.0, 0.0), Some(target)),
    ];
    let (shapes, _) = Fixture::build(Fixture::config(1, 4096), &keys, FOV, ASPECT);

    for (index, colour) in [(0, BLUE), (1, GREEN)] {
        let start = index * 9;
        let target_box = shapes[start + 8];
        assert_eq!(target_box.kind, MarkerKind::Box);
        assert!(Fixture::close(target_box.location, target - Vec3::new(0.125, 0.125, 0.125)));
        assert!(Fixture::close(target_box.end, Vec3::new(0.25, 0.25, 0.25)));
        assert!(shapes[start..start + 9].iter().all(|shape| shape.color == colour));
    }
    // no line reaches the target 200 blocks away; box edges and dashes are a few blocks at most
    let longest = shapes
        .iter()
        .filter(|shape| shape.kind == MarkerKind::Line)
        .map(|shape| (shape.end - shape.location).length())
        .fold(0.0_f32, f32::max);
    assert!(longest < 5.0, "longest line {longest}");

    let (bold, _) = Fixture::build(Fixture::config(2, 4096), &keys[..1], FOV, ASPECT);
    assert_eq!(bold.len(), 16 + 1);
    assert_eq!(bold[16].kind, MarkerKind::Box);
}

#[test]
fn full_path_fits_budget_with_reductions() {
    let keys = Fixture::line_of_keys(256, 20.0);

    let (shapes, reductions) = Fixture::build(MarkerConfig::default(), &keys, FOV, ASPECT);
    assert!(shapes.len() <= 4096, "{} shapes", shapes.len());
    assert_eq!(reductions, 2);
    assert_eq!(Fixture::count(&shapes, MarkerKind::Arrow), 255);
    assert_eq!(Fixture::count(&shapes, MarkerKind::Line), shapes.len() - 255);

    let (small, reductions) = Fixture::build(Fixture::config(2, 1200), &keys, FOV, ASPECT);
    assert!(small.len() <= 1200, "{} shapes", small.len());
    assert_eq!(reductions, 3);
    assert_eq!(Fixture::count(&small, MarkerKind::Arrow), 0);
}

#[test]
fn reductions_run_in_order_weight_dashes_arrows() {
    // 16 keys, 300 blocks of curve: 256 bold edges, 15 arrows, 353 dashes
    let keys = Fixture::line_of_keys(16, 20.0);
    let (full, none) = Fixture::build(Fixture::config(2, 4096), &keys, FOV, ASPECT);
    assert_eq!(none, 0);
    let full_dashes = full.len() - 256 - 15;

    let (weight, one) = Fixture::build(Fixture::config(2, 600), &keys, FOV, ASPECT);
    assert_eq!(one, 1);
    assert_eq!(weight.len(), 128 + full_dashes + 15);

    let (dashes, two) = Fixture::build(Fixture::config(2, 400), &keys, FOV, ASPECT);
    assert_eq!(two, 2);
    assert_eq!(dashes.len(), 400);
    assert_eq!(Fixture::count(&dashes, MarkerKind::Arrow), 15);

    let (arrows, three) = Fixture::build(Fixture::config(2, 140), &keys, FOV, ASPECT);
    assert_eq!(three, 3);
    assert_eq!(arrows.len(), 140);
    assert_eq!(Fixture::count(&arrows, MarkerKind::Arrow), 0);
}

#[test]
fn fewer_dashes_keep_their_length_and_span_the_curve() {
    let keys = Fixture::line_of_keys(16, 20.0);
    let (shapes, _) = Fixture::build(Fixture::config(2, 400), &keys, FOV, ASPECT);

    let dashes: Vec<&MarkerShape> = shapes[128..shapes.len() - 15].iter().collect();
    assert!(dashes.iter().all(|dash| ((dash.end - dash.location).length() - 0.5).abs() < 0.01));
    let last = dashes[dashes.len() - 1];
    assert!(last.location.x > 290.0, "last dash starts at {:?}", last.location);
}

#[test]
fn empty_path_builds_nothing() {
    let curve = PathCurve::new();
    let mut builder = MarkerBuilder::new(MarkerConfig::default());

    assert!(builder.build(&[], &curve, None, FOV, ASPECT).is_empty());
    assert_eq!(builder.reductions(), 0);
}

#[test]
fn disabled_markers_build_nothing() {
    let keys = Fixture::line_of_keys(3, 10.0);
    let config = MarkerConfig { enabled: false, ..MarkerConfig::default() };

    let (shapes, _) = Fixture::build(config, &keys, FOV, ASPECT);
    assert!(shapes.is_empty());
}

#[test]
fn last_key_box_survives_at_the_smallest_budget() {
    // every key a look-at key: the most fixed shapes a full path can need
    let keys: Vec<PathKey> = (0..256)
        .map(|index| {
            let position = Vec3::new(index as f32 * 20.0, 0.0, 0.0);
            PathKey::new(CameraPose::new(position, 0.0, 0.0), Some(position + Vec3::new(0.0, 0.0, 3.0)))
        })
        .collect();
    let floor = FreecamConfig { markers: Fixture::config(2, 0), ..FreecamConfig::default() }.sanitized().markers;

    let (shapes, _) = Fixture::build(floor.clone(), &keys, FOV, ASPECT);

    assert!(shapes.len() <= floor.max_shapes, "{} shapes", shapes.len());
    let green = |kind| shapes.iter().filter(|shape| shape.kind == kind && shape.color == GREEN).count();
    // 8 box edges and the target box
    assert_eq!(green(MarkerKind::Line), 8);
    assert_eq!(green(MarkerKind::Box), 1);
}
