use alaydriem_freecam::services::RenderOverride;
use freecam_core::structs::{CameraPose, Vec3};

#[test]
fn an_active_pose_is_kept_and_the_rendered_position_is_taken_once() {
    let render = RenderOverride::new();
    let pose = CameraPose::new(Vec3::new(1.0, 2.0, 3.0), 10.0, 5.0);

    render.set(Some(pose));

    assert_eq!(render.pose(), Some(pose));
    assert_eq!(render.take_rendered(), None, "nothing was written yet");
}

#[test]
fn no_pose_turns_the_override_off() {
    let render = RenderOverride::new();
    render.set(Some(CameraPose::default()));

    render.set(None);

    assert_eq!(render.pose(), None);
}

#[test]
fn reset_turns_the_override_off() {
    let render = RenderOverride::new();
    render.set(Some(CameraPose::default()));

    render.reset();

    assert_eq!(render.pose(), None);
    assert_eq!(render.take_rendered(), None);
}

#[test]
#[allow(unsafe_code)]
fn the_view_write_puts_the_position_in_the_view_object_and_keeps_it_as_rendered() {
    let writer = amml_game::CameraWriter::new(&crate::support::FakeGame::entries()).unwrap();
    let render = RenderOverride::new();
    render.set(Some(CameraPose::new(Vec3::new(7.0, 8.0, 9.0), 0.0, 0.0)));
    let mut view_object = vec![0u8; 0x200];

    // SAFETY: a buffer large enough for the fake offsets (view data 0x40, positions 0x40 into it)
    unsafe { render.apply_view(&writer, view_object.as_mut_ptr().cast()) };

    let at = |offset: usize| f32::from_le_bytes(view_object[offset..offset + 4].try_into().unwrap());
    assert_eq!([at(0x80), at(0x84), at(0x88)], [7.0, 8.0, 9.0]);
    assert_eq!(render.take_rendered(), Some(Vec3::new(7.0, 8.0, 9.0)));
    assert_eq!(render.take_rendered(), None);
}
