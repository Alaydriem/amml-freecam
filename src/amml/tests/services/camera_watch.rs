use alaydriem_freecam::services::CameraWatch;
use alaydriem_freecam::types::InstructionKind;

use crate::support::LogCapture;

const MESSAGE: &str = "the camera registry and the server camera instructions disagree";

#[test]
fn without_the_registry_the_server_instructions_decide() {
    let watch = CameraWatch::new(None);

    let before = watch.decide(None, 1, 0.0, false);
    watch.on_server_instruction(InstructionKind::Set);
    let during = watch.decide(None, 1, 0.1, false);
    watch.on_server_instruction(InstructionKind::Other);
    let after_fade = watch.decide(None, 1, 0.2, false);
    watch.on_server_instruction(InstructionKind::Clear);
    let after_clear = watch.decide(None, 1, 0.3, false);

    assert_eq!((before, during, after_fade, after_clear), (true, false, false, true));
}

#[test]
fn a_new_capture_generation_starts_with_the_player_camera() {
    let watch = CameraWatch::new(None);
    watch.decide(None, 1, 0.0, false);
    watch.on_server_instruction(InstructionKind::Set);

    assert!(!watch.decide(None, 1, 0.1, false));
    assert!(watch.decide(None, 2, 0.2, false), "a new handler");
    watch.on_server_instruction(InstructionKind::Set);
    assert!(watch.decide(None, 0, 0.3, false), "no handler: no world");
}

#[test]
fn the_registry_wins_and_a_long_disagreement_is_logged_once() {
    LogCapture::install();
    let watch = CameraWatch::new(None);

    let short = watch.decide(Some(false), 1, 0.0, false);
    let still_short = watch.decide(Some(false), 1, 0.9, false);
    let lines_short = LogCapture::count(MESSAGE);
    let long = watch.decide(Some(false), 1, 1.5, false);
    watch.decide(Some(false), 1, 3.0, false);
    let agreed = watch.decide(Some(true), 1, 3.1, false);

    assert_eq!((short, still_short, long, agreed), (false, false, false, true));
    assert_eq!(lines_short, 0);
    assert_eq!(LogCapture::count(MESSAGE), 1);
}

#[test]
fn freecam_holding_the_camera_is_not_a_disagreement() {
    LogCapture::install();
    let watch = CameraWatch::new(None);

    let held = watch.decide(Some(false), 1, 0.0, true);
    watch.decide(Some(false), 1, 2.0, true);
    let released = watch.decide(Some(true), 1, 2.1, false);

    assert_eq!((held, released), (false, true));
    assert_eq!(LogCapture::count(MESSAGE), 0);
}
