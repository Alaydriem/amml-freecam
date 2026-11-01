use std::collections::HashMap;

use freecam_core::services::MarkerSync;
use freecam_core::services::marker_sync::MARKER_ID_BASE;
use freecam_core::structs::math::Vec3;
use freecam_core::structs::{MarkerEntry, MarkerKind, MarkerShape};

const DIMENSION: i32 = 0;
const COLOR: u32 = 0xFF3D_A5FF;

struct Fixture;

impl Fixture {
    fn line(x: f32) -> MarkerShape {
        MarkerShape::new(MarkerKind::Line, Vec3::new(x, 0.0, 0.0), Vec3::new(x + 0.5, 0.0, 0.0), COLOR)
    }

    fn lines(count: usize) -> Vec<MarkerShape> {
        (0..count).map(|index| Self::line(index as f32)).collect()
    }

    // sends batches until the sync reports nothing left, and returns every batch
    fn drain(sync: &mut MarkerSync) -> Vec<Vec<MarkerEntry>> {
        let mut batches = Vec::new();
        loop {
            let batch = sync.next_batch().to_vec();
            if batch.is_empty() {
                return batches;
            }
            batches.push(batch);
        }
    }

    fn flat(batches: &[Vec<MarkerEntry>]) -> Vec<MarkerEntry> {
        batches.iter().flatten().copied().collect()
    }

    fn slot(entry: &MarkerEntry) -> u64 {
        entry.id - MARKER_ID_BASE
    }

    // what the client shows after applying every batch in order: slot to shape
    fn apply(world: &mut HashMap<u64, MarkerShape>, entries: &[MarkerEntry]) {
        for entry in entries {
            if entry.shape.kind == MarkerKind::Remove {
                world.remove(&entry.id);
            } else {
                world.insert(entry.id, entry.shape);
            }
        }
    }

    fn expected(shapes: &[MarkerShape]) -> HashMap<u64, MarkerShape> {
        shapes.iter().enumerate().map(|(slot, shape)| (MARKER_ID_BASE + slot as u64, *shape)).collect()
    }
}

#[test]
fn unchanged_target_sends_nothing() {
    let mut sync = MarkerSync::new(4096, 400);
    let shapes = Fixture::lines(10);
    sync.set_target(&shapes, DIMENSION);
    Fixture::drain(&mut sync);

    sync.set_target(&shapes, DIMENSION);

    assert!(sync.next_batch().is_empty());
    assert_eq!(sync.live_slots(), 10);
}

#[test]
fn added_point_sends_only_changed_and_new_slots() {
    let mut sync = MarkerSync::new(4096, 400);
    let mut shapes = Fixture::lines(10);
    sync.set_target(&shapes, DIMENSION);
    Fixture::drain(&mut sync);

    shapes[3] = Fixture::line(30.0);
    shapes.push(Fixture::line(40.0));
    shapes.push(Fixture::line(41.0));
    sync.set_target(&shapes, DIMENSION);
    let sent = Fixture::flat(&Fixture::drain(&mut sync));

    let slots: Vec<u64> = sent.iter().map(Fixture::slot).collect();
    assert_eq!(slots, vec![3, 10, 11]);
    assert_eq!(sent[0].shape, Fixture::line(30.0));
    assert_eq!(sent[2].shape, Fixture::line(41.0));
    assert!(sent.iter().all(|entry| entry.dimension == DIMENSION));
}

#[test]
fn kind_change_removes_the_slot_before_the_new_shape_is_sent() {
    let mut sync = MarkerSync::new(4096, 400);
    let mut shapes = Fixture::lines(4);
    sync.set_target(&shapes, DIMENSION);
    Fixture::drain(&mut sync);

    let target_box = MarkerShape::new(MarkerKind::Box, Vec3::new(9.0, 0.0, 0.0), Vec3::new(0.25, 0.25, 0.25), COLOR);
    shapes[2] = target_box;
    sync.set_target(&shapes, DIMENSION);
    let batches = Fixture::drain(&mut sync);

    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), 1);
    assert_eq!(Fixture::slot(&batches[0][0]), 2);
    assert_eq!(batches[0][0].shape.kind, MarkerKind::Remove);
    assert_eq!(batches[1].len(), 1);
    assert_eq!(Fixture::slot(&batches[1][0]), 2);
    assert_eq!(batches[1][0].shape, target_box);
    assert_eq!(sync.live_slots(), 4);
}

#[test]
fn shorter_target_removes_tail_slots() {
    let mut sync = MarkerSync::new(4096, 400);
    let shapes = Fixture::lines(10);
    sync.set_target(&shapes, DIMENSION);
    Fixture::drain(&mut sync);

    sync.set_target(&shapes[..6], DIMENSION);
    let sent = Fixture::flat(&Fixture::drain(&mut sync));

    let slots: Vec<u64> = sent.iter().map(Fixture::slot).collect();
    assert_eq!(slots, vec![6, 7, 8, 9]);
    assert!(sent.iter().all(|entry| entry.shape.kind == MarkerKind::Remove));
    assert_eq!(sync.live_slots(), 6);
}

#[test]
fn hide_removes_every_live_slot() {
    let mut sync = MarkerSync::new(4096, 400);
    sync.set_target(&Fixture::lines(25), DIMENSION);
    Fixture::drain(&mut sync);

    sync.hide();
    let sent = Fixture::flat(&Fixture::drain(&mut sync));

    assert_eq!(sent.len(), 25);
    assert!(sent.iter().all(|entry| entry.shape.kind == MarkerKind::Remove));
    assert_eq!(sync.live_slots(), 0);
}

#[test]
fn spread_never_exceeds_max_per_send() {
    let mut sync = MarkerSync::new(4096, 400);
    let shapes = Fixture::lines(4096);

    sync.set_target(&shapes, DIMENSION);
    let batches = Fixture::drain(&mut sync);

    assert_eq!(batches.len(), 11);
    assert!(batches.iter().all(|batch| batch.len() <= 400));
    let mut world = HashMap::new();
    Fixture::apply(&mut world, &Fixture::flat(&batches));
    assert_eq!(world, Fixture::expected(&shapes));
}

#[test]
fn new_target_mid_spread_leaves_no_stale_slot() {
    let mut sync = MarkerSync::new(4096, 400);
    let mut world = HashMap::new();
    sync.set_target(&Fixture::lines(2000), DIMENSION);
    for _ in 0..3 {
        let batch = sync.next_batch().to_vec();
        Fixture::apply(&mut world, &batch);
    }

    let replacement: Vec<MarkerShape> = (0..700).map(|index| Fixture::line(index as f32 + 0.25)).collect();
    sync.set_target(&replacement, DIMENSION);
    let batches = Fixture::drain(&mut sync);
    Fixture::apply(&mut world, &Fixture::flat(&batches));

    assert_eq!(world, Fixture::expected(&replacement));
    assert_eq!(sync.live_slots(), 700);
}

#[test]
fn hide_mid_spread_leaves_no_live_slot() {
    let mut sync = MarkerSync::new(4096, 400);
    let mut world = HashMap::new();
    sync.set_target(&Fixture::lines(4096), DIMENSION);
    for _ in 0..4 {
        let batch = sync.next_batch().to_vec();
        Fixture::apply(&mut world, &batch);
    }

    sync.hide();
    let batches = Fixture::drain(&mut sync);
    Fixture::apply(&mut world, &Fixture::flat(&batches));

    assert!(world.is_empty());
    assert_eq!(Fixture::flat(&batches).len(), 1600);
    assert_eq!(sync.live_slots(), 0);
}

#[test]
fn rejoin_resends_every_slot() {
    let mut sync = MarkerSync::new(4096, 400);
    let shapes = Fixture::lines(500);
    sync.set_target(&shapes, DIMENSION);
    Fixture::drain(&mut sync);

    sync.forget_sent();
    assert_eq!(sync.live_slots(), 0);
    let batches = Fixture::drain(&mut sync);

    let mut world = HashMap::new();
    Fixture::apply(&mut world, &Fixture::flat(&batches));
    assert_eq!(world, Fixture::expected(&shapes));
    assert_eq!(sync.live_slots(), 500);
}

#[test]
fn forget_sent_after_hide_sends_nothing() {
    let mut sync = MarkerSync::new(4096, 400);
    sync.set_target(&Fixture::lines(50), DIMENSION);
    Fixture::drain(&mut sync);

    sync.hide();
    sync.forget_sent();

    assert!(sync.next_batch().is_empty());
}

#[test]
fn slot_ids_start_at_marker_id_base() {
    let mut sync = MarkerSync::new(4096, 400);

    sync.set_target(&Fixture::lines(3), DIMENSION);
    let sent = sync.next_batch().to_vec();

    let ids: Vec<u64> = sent.iter().map(|entry| entry.id).collect();
    assert_eq!(MARKER_ID_BASE, 0x4643_0000_0000_0000);
    assert_eq!(ids, vec![MARKER_ID_BASE, MARKER_ID_BASE + 1, MARKER_ID_BASE + 2]);
}

#[test]
fn pending_queue_is_bounded_by_max_shapes() {
    let mut sync = MarkerSync::new(512, 400);

    for round in 0..5 {
        let shapes: Vec<MarkerShape> = (0..900).map(|index| Fixture::line(index as f32 + round as f32)).collect();
        sync.set_target(&shapes, DIMENSION);
    }
    let sent = Fixture::flat(&Fixture::drain(&mut sync));

    assert_eq!(sent.len(), 512);
    assert!(sent.iter().all(|entry| Fixture::slot(entry) < 512));
    assert_eq!(sync.live_slots(), 512);
}

#[test]
fn dimension_change_resends_live_slots() {
    let mut sync = MarkerSync::new(4096, 400);
    let shapes = Fixture::lines(5);
    sync.set_target(&shapes, DIMENSION);
    Fixture::drain(&mut sync);

    sync.set_target(&shapes, 1);
    let sent = Fixture::flat(&Fixture::drain(&mut sync));

    assert_eq!(sent.len(), 5);
    assert!(sent.iter().all(|entry| entry.dimension == 1));
}

#[test]
fn shutdown_batch_removes_every_live_slot_in_one_batch() {
    let mut sync = MarkerSync::new(4096, 400);
    sync.set_target(&Fixture::lines(1000), DIMENSION);
    Fixture::drain(&mut sync);
    sync.set_target(&Fixture::lines(4096), DIMENSION);
    sync.next_batch();

    let removes = sync.shutdown_batch().to_vec();

    assert_eq!(removes.len(), 1400);
    assert!(removes.iter().all(|entry| entry.shape.kind == MarkerKind::Remove));
    assert_eq!(sync.live_slots(), 0);
    assert!(sync.next_batch().is_empty());
}
