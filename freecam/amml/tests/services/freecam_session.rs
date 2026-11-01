use std::collections::HashSet;

use alaydriem_freecam::services::LinkSession;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use freecam_core::services::marker_sync::MARKER_ID_BASE;
use freecam_core::structs::{
    CameraCommand, CameraPose, FreecamAction, FreecamState, MarkerKind, ServerInstruction, Vec3,
};
use stats_alloc::Region;

use crate::GLOBAL;
use crate::support::SessionFixture;

fn state(fixture: &SessionFixture) -> FreecamState {
    fixture.session().with_controller(|controller| controller.state()).unwrap()
}

fn flying() -> SessionFixture {
    let fixture = SessionFixture::new();
    SessionFixture::tap(FreecamAction::Toggle);
    fixture.frame(&SessionFixture::status_at(0.05, 1));
    fixture
}

#[test]
fn toggle_then_frame_sets_the_camera_and_takes_the_flight_leases() {
    let fixture = SessionFixture::new();
    SessionFixture::tap(FreecamAction::Toggle);
    let status = SessionFixture::ready_status();

    let (_, out) = fixture.frame(&status);

    assert_eq!(out.camera, CameraCommand::Set(status.start_pose));
    assert!(out.camera_owned);
    assert_eq!(FakeTable::with(|state| state.acquired.clone()), LinkSession::FLIGHT);
}

#[test]
fn markers_wait_for_a_captured_handler() {
    let fixture = SessionFixture::with_config(SessionFixture::NO_LOOK_AT);
    let (mut time, world) = fixture.build_path(0);
    time += SessionFixture::FRAME_STEP;
    let (batch, _) = fixture.frame(&SessionFixture::status_at(time, 0));
    assert!(world.is_empty());
    assert!(batch.is_empty());

    time += SessionFixture::FRAME_STEP;
    let (batch, _) = fixture.frame(&SessionFixture::status_at(time, 1));
    assert!(!batch.is_empty());
    assert!(batch.iter().all(|entry| entry.id >= MARKER_ID_BASE && entry.shape.kind != MarkerKind::Remove));
}

#[test]
fn ray_request_comes_when_core_asks_and_its_result_reaches_core() {
    let fixture = flying();
    let (_, quiet) = fixture.frame(&SessionFixture::status_at(0.10, 1));
    assert_eq!(quiet.ray, None);

    SessionFixture::tap(FreecamAction::AddPoint);
    let (_, out) = fixture.frame(&SessionFixture::status_at(0.15, 1));
    let start = SessionFixture::start_pose();
    let forward = CameraPose::new(start.position, start.yaw, start.pitch).forward();
    let ray = out.ray.unwrap();
    assert_eq!((ray.origin, ray.direction, ray.max_distance), (start.position, forward, 256.0));

    fixture.session().ray_result(Some(Vec3::new(20.0, 64.0, -5.0))).unwrap();

    let keys = fixture.session().with_controller(|controller| controller.path().keys().to_vec()).unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].target, Some(Vec3::new(20.0, 64.0, -5.0)));
}

#[test]
fn shutdown_batch_removes_every_live_marker_once() {
    let fixture = SessionFixture::with_config(SessionFixture::NO_LOOK_AT);
    let (_, world) = fixture.build_path(1);
    assert!(!world.is_empty());
    let mut removes = Vec::new();

    fixture.session().shutdown_markers(&mut removes).unwrap();

    assert!(removes.iter().all(|entry| entry.shape.kind == MarkerKind::Remove));
    let removed: HashSet<u64> = removes.iter().map(|entry| entry.id).collect();
    let live: HashSet<u64> = world.keys().copied().collect();
    assert_eq!(removed, live);
    fixture.session().shutdown_markers(&mut removes).unwrap();
    assert!(removes.is_empty(), "nothing stays live");
}

#[test]
fn a_server_set_ends_freecam_while_no_other_mod_holds_the_camera() {
    let fixture = flying();

    fixture.session().server_instruction(ServerInstruction::SetOrClear).unwrap();
    fixture.frame(&SessionFixture::status_at(0.10, 1));

    assert_eq!(state(&fixture), FreecamState::Off);
}

#[test]
fn a_server_set_is_ignored_while_another_mod_holds_the_camera() {
    let fixture = flying();
    FakeTable::set_held_by_other(Capability::Camera, true);

    fixture.session().server_instruction(ServerInstruction::SetOrClear).unwrap();
    fixture.frame(&SessionFixture::status_at(0.10, 1));

    assert_eq!(state(&fixture), FreecamState::On);
}

#[test]
fn frame_does_not_allocate_after_warm_up() {
    let fixture = SessionFixture::with_config(SessionFixture::NO_LOOK_AT);
    SessionFixture::tap(FreecamAction::Toggle);
    let mut markers = Vec::with_capacity(fixture.session().marker_capacity());
    let mut time = 0.0;
    let mut frame = |events: usize| {
        for _ in 0..events {
            FakeTable::push_event(ModEvent::PointerDelta { dx: 1.0, dy: 0.5 });
        }
        time += SessionFixture::FRAME_STEP;
        fixture.session().frame(&SessionFixture::status_at(time, 1), &mut markers).unwrap();
    };
    // the fake manager's queue grows to its size here, so the measured frames reuse it
    frame(LinkSession::DRAIN_LIMIT);
    for _ in 0..30 {
        frame(4);
    }

    let region = Region::new(&GLOBAL);
    for _ in 0..100 {
        frame(4);
    }

    assert_eq!(region.change().allocations, 0);
}
