use amm_core::structs::event::{KeybindPhase, ModEvent};
use freecam_core::config::{FreecamConfig, LookAtConfig, MarkerConfig};
use freecam_core::services::FreecamController;
use freecam_core::structs::{
    CaptureRequest, FrameOutput, FrameStatus, FreecamAction, MarkerKind, StartLeases, ToastKind, UiAction,
};

use curia::Level;

use crate::support::frames::Frames;
use crate::support::logs::Logs;

const FRAME_STEP: f64 = 0.05;
const POINTS: usize = 3;

struct Rig {
    controller: FreecamController,
    time: f64,
}

impl Rig {
    fn with_markers(markers: MarkerConfig) -> Self {
        let config = FreecamConfig {
            markers,
            look_at: LookAtConfig { enabled: false, ..LookAtConfig::default() },
            ..FreecamConfig::default()
        };
        Self { controller: FreecamController::new(config), time: 0.0 }
    }

    fn frame_with(&mut self, edit: impl FnOnce(&mut FrameStatus)) -> FrameOutput {
        self.time += FRAME_STEP;
        let mut status = Frames::status(self.time);
        edit(&mut status);
        self.controller.frame(&status)
    }

    fn frame(&mut self) -> FrameOutput {
        self.frame_with(|_| {})
    }

    // one fc_frame call during the presented frame `present`
    fn call_in(&mut self, present: u32) -> FrameOutput {
        self.frame_with(|status| status.present_count = present)
    }

    // the frame is copied and its PNG written to `name`
    fn answer(&mut self, request: CaptureRequest, name: &str) {
        self.controller.on_capture_taken(request.sequence, true);
        self.controller.on_capture_written(request.sequence, true, name);
    }

    // the manager's keybind event, then the lease hand-off the FFI handle runs for a start request
    fn keybind(&mut self, action: FreecamAction, phase: KeybindPhase) {
        self.controller.on_link_event(&ModEvent::Keybind { action: action.index(), phase });
        if self.controller.take_start_request() {
            self.controller.finish_start(StartLeases::Granted);
        }
    }

    fn tap(&mut self, action: FreecamAction) {
        self.keybind(action, KeybindPhase::Pressed);
        self.keybind(action, KeybindPhase::Released);
    }

    // freecam on, a few points, bar mode on, markers settled in the world
    fn ready(markers: MarkerConfig) -> Self {
        Self::with_points(markers, POINTS)
    }

    // the camera walks forward between points, so the path has length
    fn with_points(markers: MarkerConfig, points: usize) -> Self {
        let mut rig = Self::with_markers(markers);
        rig.tap(FreecamAction::Toggle);
        rig.frame();
        rig.keybind(FreecamAction::FlyForward, KeybindPhase::Pressed);
        for _ in 0..points {
            rig.tap(FreecamAction::AddPoint);
            rig.frames(3);
        }
        rig.keybind(FreecamAction::FlyForward, KeybindPhase::Released);
        rig.tap(FreecamAction::ToggleBar);
        rig.frame();
        rig.frames(30);
        rig
    }

    fn frames(&mut self, count: usize) -> Vec<FrameOutput> {
        (0..count).map(|_| self.frame()).collect()
    }

    fn shoot(&mut self) {
        self.controller.apply_ui_action(UiAction::Screenshot);
    }

    // runs frames until one carries a capture request, returns it with the frame count
    fn until_capture(&mut self, limit: usize) -> Option<(usize, CaptureRequest)> {
        for index in 1..=limit {
            if let Some(request) = self.frame().capture {
                return Some((index, request));
            }
        }
        None
    }
}

#[test]
fn ui_hidden_and_markers_removed_before_capture() {
    let mut rig = Rig::ready(MarkerConfig::default());
    assert!(rig.controller.ui_snapshot().visible);

    rig.shoot();
    let first = rig.frame();

    assert!(first.hide_ui);
    assert!(!rig.controller.ui_snapshot().visible);
    assert!(first.capture.is_none());
    let batch = rig.controller.marker_batch();
    assert!(!batch.is_empty());
    assert!(batch.iter().all(|entry| entry.shape.kind == MarkerKind::Remove));

    let (_, request) = rig.until_capture(10).expect("capture request");
    assert_eq!(request.sequence, 1);
    let out = rig.frame();
    assert!(out.hide_ui);
    assert!(out.capture.is_none());
}

#[test]
fn capture_waits_for_markers_then_times_out() {
    // each send removes at most 50 shapes, so the path takes several frames to clear
    let markers = MarkerConfig { max_per_send: 50, ..MarkerConfig::default() };
    let mut rig = Rig::with_points(markers, 40);
    rig.shoot();

    let (waited, request) = rig.until_capture(200).expect("capture request");
    assert!(waited >= 4, "capture came after {waited} frame(s)");
    assert!(rig.controller.marker_batch().is_empty());

    // no taken answer: the mod manager gives a copy up to 30 frames, so the drawing returns with a Failed toast only
    // 40 presents after the request
    let hidden: Vec<bool> = (0..40).map(|_| rig.frame().hide_ui).collect();
    assert_eq!(hidden[..39], [true; 39]);
    assert!(!hidden[39]);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.toast, ToastKind::Failed);
    assert!(snapshot.visible);

    // a late answer to the dropped request changes nothing
    rig.answer(request, "late.png");
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::Failed);
}

#[test]
fn shot_skipped_when_markers_cannot_clear() {
    Logs::install();
    let mut rig = Rig::ready(MarkerConfig::default());
    let warnings = Logs::count(Level::Warn, "screenshot");
    rig.shoot();

    // the shim cannot deliver removes, so the markers stay live
    let mut shown_at = None;
    for index in 1..=30 {
        let out = rig.frame_with(|status| status.ready = false);
        assert!(out.capture.is_none(), "capture asked for at frame {index}");
        if !out.hide_ui {
            shown_at = Some(index);
            break;
        }
    }

    // exit wait (10) + ceil(4096 / 400) = 21 presents of waiting, then the drawing returns
    let shown_at = shown_at.expect("drawing returns");
    assert!(shown_at > 10 && shown_at <= 23, "drawing returned after {shown_at} frames");
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.toast, ToastKind::Failed);
    assert!(snapshot.visible);
    let later: Vec<FrameOutput> = (0..100).map(|_| rig.frame_with(|status| status.ready = false)).collect();
    assert!(later.iter().all(|out| out.capture.is_none() && !out.hide_ui));
    assert_eq!(Logs::count(Level::Warn, "screenshot"), warnings + 1);

    // the skipped shot holds nothing back: the next one starts
    rig.shoot();
    assert!(rig.frame().hide_ui);
}

#[test]
fn lost_written_answer_frees_screenshots_after_ten_seconds() {
    Logs::install();
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    let (_, request) = rig.until_capture(50).expect("capture request");
    rig.controller.on_capture_taken(request.sequence, true);
    let warnings = Logs::count(Level::Warn, "screenshot");

    // 9.75 s at 0.05 s a frame: the PNG may still be on its way, so no new shot starts yet
    rig.frames(195);
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::None);
    rig.shoot();
    assert!(!rig.frame().hide_ui);

    // past 10 s with no written answer: the Failed toast, one warning, and new shots are accepted
    rig.frames(10);
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::Failed);
    assert_eq!(Logs::count(Level::Warn, "screenshot"), warnings + 1);

    // a late written answer for that shot is ignored
    rig.controller.on_capture_written(request.sequence, true, "late.png");
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.toast, ToastKind::Failed);
    assert_eq!(snapshot.toast_file_len, 0);

    rig.shoot();
    let (_, next) = rig.until_capture(50).expect("next capture request");
    assert_eq!(next.sequence, request.sequence.wrapping_add(1));
}

#[test]
fn drawing_restored_after_result() {
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    let (_, request) = rig.until_capture(50).expect("capture request");

    rig.answer(request, "freecam-20261020-120000.png");
    let out = rig.frame();

    assert!(!out.hide_ui);
    assert!(out.capture.is_none());
    let snapshot = rig.controller.ui_snapshot();
    assert!(snapshot.visible);
    assert_eq!(snapshot.toast, ToastKind::Saved);
    assert_eq!(&snapshot.toast_file[..snapshot.toast_file_len as usize], b"freecam-20261020-120000.png");

    // markers come back into the world
    let batch = rig.controller.marker_batch();
    assert!(!batch.is_empty());
    assert!(batch.iter().all(|entry| entry.shape.kind != MarkerKind::Remove));
}

#[test]
fn second_request_while_pending_is_refused() {
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    rig.shoot();
    let (_, request) = rig.until_capture(50).expect("capture request");
    rig.shoot();

    // the request goes out once only
    for out in rig.frames(5) {
        assert!(out.capture.is_none());
    }
    // a result for another sequence is not the answer
    rig.answer(CaptureRequest { sequence: request.sequence.wrapping_add(1) }, "other.png");
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::None);
    assert!(rig.frame().hide_ui);
}

#[test]
fn toast_shows_for_three_seconds() {
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    let (_, request) = rig.until_capture(50).expect("capture request");
    rig.answer(request, "a.png");

    // 3.0 s at 0.05 s a frame is 60 frames
    rig.frames(55);
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::Saved);
    rig.frames(10);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.toast, ToastKind::None);
    assert_eq!(snapshot.toast_file_len, 0);
}

#[test]
fn toast_file_name_is_cut_on_a_character_boundary() {
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    let (_, request) = rig.until_capture(50).expect("capture request");
    // 62 ASCII bytes then a 3-byte character: cutting at 63 bytes would split it
    let name = format!("{}\u{20AC}tail", "a".repeat(62));
    rig.answer(request, &name);

    let snapshot = rig.controller.ui_snapshot();
    let shown = std::str::from_utf8(&snapshot.toast_file[..snapshot.toast_file_len as usize]).expect("utf-8");
    assert_eq!(shown, "a".repeat(62));
}

#[test]
fn failed_result_shows_failed_toast() {
    let mut refused = Rig::ready(MarkerConfig::default());
    refused.shoot();
    let (_, request) = refused.until_capture(50).expect("capture request");
    refused.controller.on_capture_taken(request.sequence, false);
    // a refused frame has no PNG, so a written answer for it is not waited for
    refused.controller.on_capture_written(request.sequence, true, "ignored.png");

    let mut unwritten = Rig::ready(MarkerConfig::default());
    unwritten.shoot();
    let (_, request) = unwritten.until_capture(50).expect("capture request");
    unwritten.controller.on_capture_taken(request.sequence, true);
    unwritten.controller.on_capture_written(request.sequence, false, "ignored.png");

    for rig in [refused, unwritten] {
        let snapshot = rig.controller.ui_snapshot();
        assert_eq!(snapshot.toast, ToastKind::Failed);
        assert_eq!(snapshot.toast_file_len, 0);
    }
}

#[test]
fn toast_waits_for_written_after_taken() {
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    let (_, request) = rig.until_capture(50).expect("capture request");

    rig.controller.on_capture_taken(request.sequence, true);
    let after_taken = rig.frame();
    assert!(!after_taken.hide_ui);
    assert!(rig.controller.ui_snapshot().visible);
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::None);

    // the PNG is still being written: no new shot starts, and the frame timeout no longer applies
    rig.shoot();
    assert!(rig.frames(30).iter().all(|out| !out.hide_ui && out.capture.is_none()));
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::None);

    // a written answer for another sequence is not the answer
    rig.controller.on_capture_written(request.sequence.wrapping_add(1), true, "other.png");
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::None);

    rig.controller.on_capture_written(request.sequence, true, "b.png");
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.toast, ToastKind::Saved);
    assert_eq!(&snapshot.toast_file[..snapshot.toast_file_len as usize], b"b.png");

    // once written, the next shot may start
    rig.shoot();
    assert!(rig.frame().hide_ui);
}

#[test]
fn capture_waits_two_presents_after_markers_leave() {
    // each send removes at most 50 shapes, so the removes take several presents
    let markers = MarkerConfig { max_per_send: 50, ..MarkerConfig::default() };
    let mut rig = Rig::with_points(markers, 40);
    let mut present = 1_000_000;
    rig.shoot();

    // two fc_frame calls per presented frame, as the game makes them
    let mut last_remove_present = None;
    let mut capture = None;
    'presents: for _ in 0..200 {
        present += 1;
        for _ in 0..2 {
            if let Some(request) = rig.call_in(present).capture {
                capture = Some((present, request));
                break 'presents;
            }
            if rig.controller.marker_batch().iter().any(|entry| entry.shape.kind == MarkerKind::Remove) {
                last_remove_present = Some(present);
            }
        }
    }
    let (capture_present, _) = capture.expect("capture request");
    let last_remove_present = last_remove_present.expect("removes went out");
    // fc_frame and Present run pipelined, so the capture is asked for two presented frames after the one that
    // sent the last removes, however many fc_frame calls each present has
    assert_eq!(
        capture_present,
        last_remove_present + 2,
        "removes at {last_remove_present}, capture at {capture_present}"
    );

    // the timeout is 40 presents, so 80 calls here; the second call of the capture's present starts it
    let mut hidden = Vec::new();
    for offset in 0..42 {
        let calls = if offset == 0 { 1 } else { 2 };
        for _ in 0..calls {
            hidden.push(rig.call_in(capture_present + offset).hide_ui);
        }
    }
    let shown_at = hidden.iter().position(|hide| !hide).expect("drawing returns");
    assert_eq!(shown_at, 79, "{hidden:?}");
}

#[test]
fn screenshot_outside_bar_mode_or_playback_is_ignored() {
    let mut off = Rig::with_markers(MarkerConfig::default());
    off.shoot();
    assert!(!off.frame().hide_ui);

    let mut rig = Rig::ready(MarkerConfig::default());
    rig.controller.apply_ui_action(UiAction::Play);
    rig.frame();
    rig.shoot();
    assert!(!rig.frame().hide_ui);
    assert!(rig.frames(30).iter().all(|out| out.capture.is_none()));
}

#[test]
fn leaving_freecam_drops_the_shot() {
    let mut rig = Rig::ready(MarkerConfig::default());
    rig.shoot();
    rig.frame();
    rig.tap(FreecamAction::Toggle);
    let outputs = rig.frames(40);

    assert!(outputs.iter().all(|out| out.capture.is_none()));
    assert!(outputs.last().is_some_and(|out| !out.hide_ui));
    assert_eq!(rig.controller.ui_snapshot().toast, ToastKind::None);
}
