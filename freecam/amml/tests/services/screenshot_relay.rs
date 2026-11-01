use std::path::Path;

use alaydriem_freecam::services::FreecamSession;
use amm_abi::types::{AmmApi, AmmStatus};
use amm_client::link::ApiTable;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::ModEvent;
use freecam_core::structs::{FreecamAction, ToastKind, UiAction, UiSnapshot};

use crate::support::SessionFixture;

const SEQUENCE: u32 = 77;
const FRAMES: usize = 60;

struct Shot {
    session: SessionFixture,
    dir: tempfile::TempDir,
    time: f64,
}

impl Shot {
    // freecam on in bar mode, with screenshots going to a temporary folder
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_str().unwrap().replace('\\', "\\\\");
        let session = SessionFixture::with_config(&format!("screenshot {{\n  folder = \"{folder}\"\n}}\n"));
        let mut shot = Self { session, dir, time: 0.0 };
        SessionFixture::tap(FreecamAction::Toggle);
        shot.frames(2);
        SessionFixture::tap(FreecamAction::ToggleBar);
        shot.frames(2);
        shot
    }

    // one drawn frame: the manager calls the frame handler, then the game calls fc_frame
    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            FakeTable::run_frame();
            self.time += SessionFixture::FRAME_STEP;
            self.session.frame(&SessionFixture::status_at(self.time, 1));
        }
    }

    fn click_screenshot(&self) {
        self.session.session().with_controller(|controller| controller.apply_ui_action(UiAction::Screenshot)).unwrap();
    }

    fn requests() -> Vec<String> {
        FakeTable::with(|state| state.screenshot_requests.clone())
    }

    // drawn frames until the manager got a request, at most FRAMES
    fn until_requested(&mut self) -> Vec<String> {
        for _ in 0..FRAMES {
            if !Self::requests().is_empty() {
                break;
            }
            self.frames(1);
        }
        Self::requests()
    }

    fn snapshot(&self) -> UiSnapshot {
        self.session.session().with_controller(|controller| controller.ui_snapshot()).unwrap()
    }
}

#[test]
fn screenshot_requests_one_capture_and_toasts_the_name() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, SEQUENCE);
    let mut shot = Shot::new();
    assert!(shot.snapshot().can_screenshot);

    shot.click_screenshot();
    let requests = shot.until_requested();
    FakeTable::push_event(ModEvent::ScreenshotTaken { sequence: SEQUENCE, ok: true });
    FakeTable::push_event(ModEvent::ScreenshotWritten { sequence: SEQUENCE, ok: true });
    shot.frames(1);

    assert_eq!(requests.len(), 1);
    let path = Path::new(&requests[0]);
    assert_eq!(path.parent(), Some(shot.dir.path()));
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("freecam-") && name.ends_with(".png"), "{name}");
    let snapshot = shot.snapshot();
    assert_eq!(snapshot.toast, ToastKind::Saved);
    assert_eq!(&snapshot.toast_file[..usize::from(snapshot.toast_file_len)], name.as_bytes());
    assert!(snapshot.visible);
}

#[test]
fn failed_screenshot_shows_ui_and_failure_toast() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, SEQUENCE);
    let mut shot = Shot::new();
    shot.click_screenshot();
    shot.until_requested();

    FakeTable::push_event(ModEvent::ScreenshotTaken { sequence: SEQUENCE, ok: true });
    FakeTable::push_event(ModEvent::ScreenshotWritten { sequence: SEQUENCE, ok: false });
    shot.frames(1);

    let snapshot = shot.snapshot();
    assert_eq!(snapshot.toast, ToastKind::Failed);
    assert!(snapshot.visible);
}

#[test]
fn refused_request_shows_the_failure_toast_at_once() {
    FakeTable::set_screenshot_answer(AmmStatus::Busy, 0);
    let mut shot = Shot::new();
    shot.click_screenshot();

    shot.until_requested();
    shot.frames(1);

    let snapshot = shot.snapshot();
    assert_eq!(snapshot.toast, ToastKind::Failed);
    assert!(snapshot.visible);
    assert!(snapshot.can_screenshot);
}

#[test]
fn answers_for_another_sequence_change_nothing() {
    FakeTable::set_screenshot_answer(AmmStatus::Ok, SEQUENCE);
    let mut shot = Shot::new();
    shot.click_screenshot();
    shot.until_requested();

    FakeTable::push_event(ModEvent::ScreenshotTaken { sequence: SEQUENCE + 1, ok: true });
    FakeTable::push_event(ModEvent::ScreenshotWritten { sequence: SEQUENCE + 1, ok: true });
    shot.frames(1);

    assert_eq!(shot.snapshot().toast, ToastKind::None);
    assert!(!shot.snapshot().visible);
}

#[test]
fn unsupported_request_turns_the_button_off() {
    FakeTable::set_screenshot_answer(AmmStatus::Unsupported, 0);
    let mut shot = Shot::new();
    shot.click_screenshot();

    shot.until_requested();
    shot.frames(1);

    assert!(!shot.snapshot().can_screenshot);
}

#[test]
fn screenshot_hidden_without_m9_slot() {
    let api = AmmApi { request_screenshot: None, held_by_other: None, texture_budget: None, ..FakeTable::api() };
    // SAFETY: the table is leaked, so it stays valid for the process; the missing slots are optional ones
    let table = unsafe { ApiTable::new(FakeTable::leak(api)) }.unwrap();

    let handle = FreecamSession::create(table, Path::new(FakeTable::CONFIG_DIR)).unwrap();

    let can_screenshot = handle.with_controller(|controller| controller.ui_snapshot().can_screenshot).unwrap();
    assert!(!can_screenshot);
}
