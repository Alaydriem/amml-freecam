use std::sync::Arc;

use alaydriem_freecam::game::{PacketHandler, ShapeLayout};
use alaydriem_freecam::services::{PacketCapture, ShapeDelivery};
use freecam_core::structs::{MarkerEntry, MarkerKind, MarkerShape, Vec3};

use crate::support::{FakeGame, FakeHeap};

fn delivery() -> ShapeDelivery {
    let entries = FakeGame::entries();
    ShapeDelivery::new(ShapeLayout::new(&entries).unwrap(), PacketHandler::new(&entries).unwrap(), FakeHeap::msvc())
}

fn captured() -> PacketCapture {
    let capture = PacketCapture::new();
    capture.capture(FakeGame::handler(), FakeGame::network_id());
    capture
}

fn marker(id: u64, kind: MarkerKind) -> MarkerEntry {
    MarkerEntry::new(id, MarkerShape::new(kind, Vec3::new(0.0, 64.0, 0.0), Vec3::new(1.0, 65.0, 1.0), 0xFFFF_FFFF), 0)
}

#[test]
fn a_batch_reaches_the_handler_in_order_with_each_type() {
    let shapes = delivery();
    let capture = captured();
    let batch = [marker(1, MarkerKind::Line), marker(2, MarkerKind::Box), marker(3, MarkerKind::Arrow)];

    assert!(shapes.deliver(&capture, &batch));

    let seen = FakeGame::shape_batches();
    assert_eq!(seen.len(), 1);
    let ids: Vec<u64> = seen[0].iter().map(|shape| shape.id).collect();
    let types: Vec<Option<u8>> = seen[0].iter().map(|shape| shape.shape_type).collect();
    let extras: Vec<u8> = seen[0].iter().map(|shape| shape.extra_index).collect();
    assert_eq!(ids, [1, 2, 3]);
    assert_eq!(types, [Some(ShapeLayout::SHAPE_LINE), Some(ShapeLayout::SHAPE_BOX), Some(ShapeLayout::SHAPE_ARROW)]);
    assert_eq!(extras, [ShapeLayout::EXTRA_LINE, ShapeLayout::EXTRA_BOX, ShapeLayout::EXTRA_ARROW]);
}

#[test]
fn removes_carry_no_shape_type() {
    let shapes = delivery();
    let capture = captured();

    shapes.deliver(&capture, &[marker(9, MarkerKind::Remove)]);

    assert_eq!(FakeGame::shape_batches()[0][0].shape_type, None);
}

#[test]
fn a_larger_batch_grows_the_block_and_frees_the_old_one() {
    let shapes = delivery();
    let capture = captured();

    shapes.deliver(&capture, &[marker(1, MarkerKind::Line)]);
    let frees_before = FakeHeap::frees();
    shapes.deliver(&capture, &[marker(1, MarkerKind::Line), marker(2, MarkerKind::Line)]);
    shapes.deliver(&capture, &[marker(3, MarkerKind::Line)]);

    assert_eq!(FakeHeap::frees(), frees_before + 1);
    let sizes: Vec<usize> = FakeGame::shape_batches().iter().map(Vec::len).collect();
    assert_eq!(sizes, [1, 2, 1]);
    assert_eq!(FakeGame::creates(), 1, "one packet serves the session");
}

#[test]
fn nothing_goes_out_without_a_captured_handler_or_entries() {
    let shapes = delivery();

    assert!(!shapes.deliver(&PacketCapture::new(), &[marker(1, MarkerKind::Line)]));
    assert!(!shapes.deliver(&captured(), &[]));
    assert_eq!(FakeGame::creates(), 0);
}

// Review Focus 4: a delivery on another thread is refused and calls nothing
#[test]
fn a_batch_on_another_thread_is_refused() {
    let shapes = Arc::new(delivery());
    let capture = Arc::new(captured());

    let delivered = {
        let (shapes, capture) = (Arc::clone(&shapes), Arc::clone(&capture));
        std::thread::spawn(move || shapes.deliver(&capture, &[marker(1, MarkerKind::Line)])).join().unwrap()
    };

    assert!(!delivered);
    assert_eq!(FakeGame::creates(), 0);
    assert!(FakeGame::shape_batches().is_empty());
    assert_eq!(capture.refusals(), 1);
}

#[test]
fn reset_gives_the_packet_back() {
    let shapes = delivery();
    let capture = captured();
    shapes.deliver(&capture, &[marker(1, MarkerKind::Line)]);

    shapes.reset();

    assert_eq!(FakeGame::destroys(), 1);
}
