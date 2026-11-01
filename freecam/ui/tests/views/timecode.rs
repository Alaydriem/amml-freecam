use freecam_ui::views::Timecode;

#[test]
fn timecode_rounds_to_hundredths_and_carries_into_minutes() {
    assert_eq!(Timecode::short(65.432), "01:05.43");
    assert_eq!(Timecode::short(59.999), "01:00.00");
    assert_eq!(Timecode::short(-1.0), "00:00.00");
    assert_eq!(Timecode::short(f32::NAN), "00:00.00");
}
