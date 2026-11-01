use alaydriem_freecam::services::LinkSession;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::lease::AcquireOutcome;
use amm_core::structs::registry::ModIndex;
use amml_i18n::text::Text;
use freecam_core::structs::{CameraCommand, FreecamAction, FreecamState, StartLeases};
use freecam_ui::message::FreecamMessage;

use crate::support::SessionFixture;

const EXIT_WAIT: usize = 10;

fn state(session: &SessionFixture) -> FreecamState {
    session.session().with_controller(|controller| controller.state()).unwrap()
}

fn position(session: &SessionFixture, time: f64) -> (f32, f32, f32) {
    let (_, out) = session.frame(&SessionFixture::status_at(time, 1));
    let pose = out.render_override.unwrap().position;
    (pose.x, pose.y, pose.z)
}

#[test]
fn fly_keybind_events_reach_the_controller() {
    let session = SessionFixture::with_config("motion {\n  smooth_at_start = false\n}\n");
    SessionFixture::tap(FreecamAction::Toggle);
    let start = position(&session, 0.05);

    SessionFixture::keybind(FreecamAction::FlyForward, KeybindPhase::Pressed);
    let moved = position(&session, 0.10);

    assert_ne!(moved, start);
}

#[test]
fn denied_lease_releases_granted_ones_and_stays_off() {
    let session = SessionFixture::new();
    FakeTable::set_acquire_outcome(Capability::Input, AcquireOutcome::Denied { holder: ModIndex(3) });
    SessionFixture::tap(FreecamAction::Toggle);

    let (_, out) = session.frame(&SessionFixture::ready_status());

    assert_eq!(FakeTable::with(|state| state.acquired.clone()), [Capability::Camera, Capability::Input]);
    assert_eq!(FakeTable::with(|state| state.released.clone()), [Capability::Camera]);
    assert_eq!(out.camera, CameraCommand::None);
    assert_eq!(state(&session), FreecamState::Off);
}

#[test]
fn suspended_camera_releases_every_lease() {
    let session = SessionFixture::new();
    SessionFixture::tap(FreecamAction::Toggle);
    session.frame(&SessionFixture::status_at(0.05, 1));

    FakeTable::push_event(ModEvent::Suspended { capability: Capability::Camera, by: ModIndex(1) });
    let mut time = 0.05;
    for _ in 0..EXIT_WAIT + 1 {
        time += SessionFixture::FRAME_STEP;
        session.frame(&SessionFixture::status_at(time, 1));
    }

    assert_eq!(state(&session), FreecamState::Off);
    assert_eq!(FakeTable::with(|state| state.released.clone()), LinkSession::FLIGHT);
}

#[test]
fn freecam_off_by_toggle_gives_the_leases_back_once() {
    let session = SessionFixture::new();
    SessionFixture::tap(FreecamAction::Toggle);
    session.frame(&SessionFixture::status_at(0.05, 1));
    SessionFixture::tap(FreecamAction::Toggle);

    let mut time = 0.05;
    for _ in 0..EXIT_WAIT + 5 {
        time += SessionFixture::FRAME_STEP;
        session.frame(&SessionFixture::status_at(time, 1));
    }

    assert_eq!(FakeTable::with(|state| state.released.clone()), LinkSession::FLIGHT);
}

#[test]
fn disabled_forgets_the_leases_without_releasing() {
    let session = SessionFixture::new();
    SessionFixture::tap(FreecamAction::Toggle);
    session.frame(&SessionFixture::status_at(0.05, 1));

    FakeTable::push_event(ModEvent::Disabled);
    let (_, out) = session.frame(&SessionFixture::status_at(0.10, 1));

    assert_eq!(out.camera, CameraCommand::Clear);
    assert_eq!(state(&session), FreecamState::Off);
    assert!(FakeTable::with(|state| state.released.is_empty()));
}

#[test]
fn cursor_mode_sent_only_on_change() {
    let mut session = LinkSession::new(FakeTable::link_without_frame());

    let sent: Vec<bool> =
        [true, true, true, false, false, true].into_iter().map(|on| session.sync_cursor(on)).collect();

    assert_eq!(sent, [true, false, false, true, false, true]);
}

#[test]
fn acquire_flight_counts_preempted_as_granted() {
    let mut session = LinkSession::new(FakeTable::link_without_frame());
    FakeTable::set_acquire_outcome(Capability::Hud, AcquireOutcome::Preempted);

    assert_eq!(session.acquire_flight(), StartLeases::Granted);
    assert!(session.holding());
}

#[test]
fn drain_is_bounded() {
    let mut session = LinkSession::new(FakeTable::link_without_frame());
    for _ in 0..LinkSession::DRAIN_LIMIT + 10 {
        FakeTable::push_event(ModEvent::Wheel { delta: 1.0 });
    }

    let first = session.drain().len();
    let second = session.drain().len();

    assert_eq!(first, LinkSession::DRAIN_LIMIT);
    assert_eq!(second, 10);
}

#[test]
fn freecam_reads_the_manager_language_when_it_registers() {
    FakeTable::with(|state| state.locale = "en_XA");

    let _session = SessionFixture::new();

    assert_eq!(FakeTable::with(|state| state.locale_reads), 1);
}

#[test]
fn apply_locale_gives_the_layers_the_words_of_the_manager_language() {
    let session = LinkSession::new(FakeTable::link_without_frame());
    let english = Text::message(FreecamMessage::AddPoint).resolve(&session.localizer()).into_owned();
    FakeTable::with(|state| state.locale = "en_XA");

    session.apply_locale();

    let pseudo = Text::message(FreecamMessage::AddPoint).resolve(&session.localizer()).into_owned();
    assert_eq!(english, "Add point");
    assert_ne!(pseudo, english, "the layers still have English");
}

#[test]
fn locale_changed_reads_the_language_again() {
    let session = SessionFixture::new();
    FakeTable::with(|state| state.locale = "en_XA");

    FakeTable::push_event(ModEvent::LocaleChanged);
    session.frame(&SessionFixture::ready_status());

    assert_eq!(FakeTable::with(|state| state.locale_reads), 2);
}

#[test]
fn exit_after_another_mod_took_the_camera_sends_no_clear() {
    let session = SessionFixture::new();
    SessionFixture::tap(FreecamAction::Toggle);
    session.frame(&SessionFixture::status_at(0.05, 1));
    FakeTable::push_event(ModEvent::Suspended { capability: Capability::Camera, by: ModIndex(1) });
    FakeTable::set_held_by_other(Capability::Camera, true);

    let mut time = 0.05;
    let mut cameras = Vec::new();
    for _ in 0..EXIT_WAIT + 2 {
        time += SessionFixture::FRAME_STEP;
        cameras.push(session.frame(&SessionFixture::status_at(time, 1)).1.camera);
    }

    assert!(cameras.iter().all(|camera| *camera == CameraCommand::None), "{cameras:?}");
    assert_eq!(state(&session), FreecamState::Off);
}
