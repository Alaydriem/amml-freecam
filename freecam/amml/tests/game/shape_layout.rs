use alaydriem_freecam::game::ShapeLayout;
use freecam_core::structs::{MarkerEntry, MarkerKind, MarkerShape, Vec3};

use crate::support::FakeGame;

fn entry(kind: MarkerKind) -> MarkerEntry {
    MarkerEntry::new(77, MarkerShape::new(kind, Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0), 0xFF33_6699), -1)
}

fn filled(kind: MarkerKind) -> Vec<u8> {
    let layout = ShapeLayout::new(&FakeGame::entries()).unwrap();
    let mut payload = vec![0u8; ShapeLayout::SIZE];
    // SAFETY: the buffer is one zeroed payload
    unsafe { layout.fill(payload.as_mut_ptr(), &entry(kind)) };
    payload
}

fn f32_at(bytes: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn a_remove_carries_the_id_only() {
    let payload = filled(MarkerKind::Remove);

    assert_eq!(u64::from_le_bytes(payload[0..8].try_into().unwrap()), 77);
    assert!(payload[8..].iter().all(|byte| *byte == 0));
}

#[test]
fn a_line_has_its_type_location_color_dimension_and_end() {
    let payload = filled(MarkerKind::Line);

    assert_eq!((payload[8], payload[9]), (ShapeLayout::SHAPE_LINE, 1));
    assert_eq!([f32_at(&payload, 0x0C), f32_at(&payload, 0x10), f32_at(&payload, 0x14)], [1.0, 2.0, 3.0]);
    assert_eq!(payload[0x18], 1);
    assert_eq!((f32_at(&payload, 0x2C), payload[0x30]), (1.0, 1));
    assert_eq!(i32::from_le_bytes(payload[0x50..0x54].try_into().unwrap()), -1);
    assert_eq!(payload[0xB0], ShapeLayout::EXTRA_LINE);
    assert_eq!([f32_at(&payload, 0x70), f32_at(&payload, 0x74), f32_at(&payload, 0x78)], [4.0, 5.0, 6.0]);
}

#[test]
fn an_arrow_has_its_head() {
    let payload = filled(MarkerKind::Arrow);

    assert_eq!(payload[8], ShapeLayout::SHAPE_ARROW);
    assert_eq!(payload[0xB0], ShapeLayout::EXTRA_ARROW);
    assert_eq!(payload[0x7C], 1);
    assert_eq!((f32_at(&payload, 0x80), payload[0x84]), (ShapeLayout::ARROW_HEAD_LENGTH, 1));
    assert_eq!((f32_at(&payload, 0x88), payload[0x8C]), (ShapeLayout::ARROW_HEAD_RADIUS, 1));
    assert_eq!((payload[0x90], payload[0x91]), (ShapeLayout::ARROW_HEAD_SEGMENTS, 1));
}

#[test]
fn argb_becomes_rgba_floats() {
    let [r, g, b, a] = ShapeLayout::color(0x80FF_0033);

    assert_eq!((r, g, a), (1.0, 0.0, 128.0 / 255.0));
    assert!((b - 0.2).abs() < 1e-6);
}
