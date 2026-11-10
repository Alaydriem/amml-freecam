use alaydriem_freecam::types::RayBox;

const UNIT: [f32; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];

#[test]
fn a_segment_through_the_box_hits_its_near_face() {
    let hit = RayBox::from_bounds(UNIT).hit([-2.0, 0.5, 0.5], [3.0, 0.5, 0.5]);

    assert_eq!(hit, Some([0.0, 0.5, 0.5]));
}

#[test]
fn a_segment_that_passes_by_misses() {
    assert_eq!(RayBox::from_bounds(UNIT).hit([-2.0, 2.0, 0.5], [3.0, 2.0, 0.5]), None);
}

#[test]
fn a_segment_that_ends_before_the_box_misses() {
    assert_eq!(RayBox::from_bounds(UNIT).hit([-2.0, 0.5, 0.5], [-0.5, 0.5, 0.5]), None);
}

#[test]
fn a_start_inside_the_box_hits_at_the_start() {
    assert_eq!(RayBox::from_bounds(UNIT).hit([0.5, 0.5, 0.5], [3.0, 3.0, 3.0]), Some([0.5, 0.5, 0.5]));
}

#[test]
fn the_margin_grows_every_face() {
    let grown = RayBox::from_bounds(UNIT).grown(RayBox::DEFAULT_MARGIN);

    let hit = grown.hit([-2.0, 0.5, 0.5], [3.0, 0.5, 0.5]).unwrap();

    assert!((hit[0] + 0.1).abs() < 1e-6, "{hit:?}");
    assert!(grown.contains([1.05, 0.5, 0.5]));
}

#[test]
fn the_search_box_spans_the_segment() {
    let span = RayBox::spanning([5.0, -1.0, 2.0], [1.0, 3.0, 2.0]);

    assert_eq!(span.bounds(), [1.0, -1.0, 2.0, 5.0, 3.0, 2.0]);
}
