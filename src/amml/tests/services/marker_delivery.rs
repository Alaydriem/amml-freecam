use alaydriem_freecam::services::MarkerDelivery;
use amml_api::{Rgba, ShapeOp, Vec3 as ApiVec3};
use freecam_core::structs::{MarkerEntry, MarkerKind, MarkerShape, Vec3};
use stats_alloc::Region;

fn entry(id: u64, kind: MarkerKind, dimension: i32) -> MarkerEntry {
    MarkerEntry::new(
        id,
        MarkerShape::new(kind, Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0), 0x80FF_4020),
        dimension,
    )
}

fn collect(delivery: &mut MarkerDelivery, markers: &[MarkerEntry], max_batch: usize) -> Vec<(i32, Vec<ShapeOp>)> {
    let mut batches = Vec::new();
    delivery.for_each_batch(markers, max_batch, |dimension, ops| batches.push((dimension, ops.to_vec())));
    batches
}

fn api(x: f32, y: f32, z: f32) -> ApiVec3 {
    ApiVec3 { x, y, z }
}

#[test]
fn each_marker_kind_becomes_its_shape_op_with_the_argb_colour() {
    let color = Rgba { r: 0xFF, g: 0x40, b: 0x20, a: 0x80 };
    let markers = [
        entry(1, MarkerKind::Line, 0),
        entry(2, MarkerKind::Box, 0),
        entry(3, MarkerKind::Arrow, 0),
        entry(4, MarkerKind::Remove, 0),
    ];

    let batches = collect(&mut MarkerDelivery::new(), &markers, 800);

    assert_eq!(
        batches,
        vec![(
            0,
            vec![
                ShapeOp::Line { id: 1, from: api(1.0, 2.0, 3.0), to: api(4.0, 5.0, 6.0), color },
                ShapeOp::Box { id: 2, from: api(1.0, 2.0, 3.0), to: api(4.0, 5.0, 6.0), color },
                ShapeOp::Arrow { id: 3, from: api(1.0, 2.0, 3.0), to: api(4.0, 5.0, 6.0), color },
                ShapeOp::Remove { id: 4 },
            ]
        )]
    );
}

#[test]
fn a_batch_splits_at_a_dimension_change_and_at_the_largest_batch() {
    let markers = [
        entry(1, MarkerKind::Remove, 0),
        entry(2, MarkerKind::Remove, 0),
        entry(3, MarkerKind::Remove, 0),
        entry(4, MarkerKind::Remove, 1),
    ];

    let batches = collect(&mut MarkerDelivery::new(), &markers, 2);

    let shape: Vec<(i32, usize)> = batches.iter().map(|(dimension, ops)| (*dimension, ops.len())).collect();
    assert_eq!(shape, vec![(0, 2), (0, 1), (1, 1)]);
}

#[test]
fn no_markers_give_no_batch() {
    assert!(collect(&mut MarkerDelivery::new(), &[], 800).is_empty());
}

#[test]
fn a_delivery_after_the_first_reuses_its_buffer() {
    let markers: Vec<MarkerEntry> = (0..64).map(|id| entry(id, MarkerKind::Line, 0)).collect();
    let mut delivery = MarkerDelivery::new();
    let mut count = 0usize;
    delivery.for_each_batch(&markers, 800, |_, ops| count += ops.len());

    let region = Region::new(crate::GLOBAL);
    delivery.for_each_batch(&markers, 800, |_, ops| count += ops.len());
    let stats = region.change();

    assert_eq!(count, 128);
    assert_eq!(stats.allocations, 0, "{stats:?}");
}
