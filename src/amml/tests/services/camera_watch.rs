use alaydriem_freecam::services::CameraWatch;
use amml_api::{CameraKind, InstructionKind};

#[test]
fn a_vanilla_camera_is_a_player_camera() {
    let mut watch = CameraWatch::new();

    assert!(watch.player_camera_active(Some(CameraKind::FirstPerson)));
    assert!(watch.player_camera_active(Some(CameraKind::ThirdPerson)));
    assert!(watch.player_camera_active(Some(CameraKind::ThirdPersonFront)));
    assert!(!watch.player_camera_active(Some(CameraKind::Other)));
}

#[test]
fn without_a_read_the_server_instructions_decide() {
    let mut watch = CameraWatch::new();
    assert!(watch.player_camera_active(None));

    watch.on_server_instruction(InstructionKind::Set);
    assert!(!watch.player_camera_active(None));

    watch.on_server_instruction(InstructionKind::Clear);
    assert!(watch.player_camera_active(None));
}

#[test]
fn a_world_leave_forgets_the_server_camera() {
    let mut watch = CameraWatch::new();
    watch.on_server_instruction(InstructionKind::Set);

    watch.reset();

    assert!(watch.player_camera_active(None));
}

#[test]
fn a_kind_change_is_reported_once() {
    let mut watch = CameraWatch::new();

    watch.player_camera_active(Some(CameraKind::FirstPerson));
    let first = watch.take_changed();
    watch.player_camera_active(Some(CameraKind::FirstPerson));
    let same = watch.take_changed();
    watch.player_camera_active(Some(CameraKind::ThirdPerson));
    let changed = watch.take_changed();

    assert_eq!(first, Some("first_person"));
    assert_eq!(same, None);
    assert_eq!(changed, Some("third_person"));
}
