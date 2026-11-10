use std::time::Instant;

use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};

use crate::config::FreecamConfig;
use crate::services::path_store::PATH_CAPACITY;
use crate::services::{
    BarEditor, CameraMotion, InputTracker, LookDrag, MarkerBuilder, MarkerSync, PathSession, ScreenshotSequence,
    SmoothnessTelemetry, Toast,
};
use crate::structs::{
    CameraCommand, CameraPose, FrameOutput, FrameStatus, FreecamAction, FreecamState, KeyEdit, MarkerEntry,
    ScreenshotStep, ServerInstruction, StartLeases, StartRefusal, TelemetryReport, ToastKind, UiAction, UiLegend,
    UiSnapshot, UiTimingSummary, Vec3,
};

// relative FOV or aspect change that resizes the marker boxes
const VIEW_CHANGE: f32 = 0.01;

pub struct FreecamController {
    motion: CameraMotion,
    input: InputTracker,
    telemetry: SmoothnessTelemetry,
    path: PathSession,
    exit_wait_frames: u32,
    state: FreecamState,
    menu_open: bool,
    toggle_requested: bool,
    // a toggle while off waits for the mod layer to get the flight leases
    start_requested: bool,
    handover_requested: bool,
    // the mod manager suspended or took back the Camera or Input lease
    lease_lost: bool,
    // the mod manager disabled freecam and took back every lease
    disabled: bool,
    play_requested: bool,
    last_time: Option<f64>,
    health: i32,
    dimension: i32,
    legend: UiLegend,
    // raised each time the legend is built again from a new config
    legend_revision: u64,
    cursor_requested: bool,
    cursor_mode: bool,
    drag: LookDrag,
    pointer_over_ui: bool,
    editor: BarEditor,
    // true when bar mode comes back once the playback it started is over
    bar_resume: bool,
    last_report: Option<TelemetryReport>,
    markers_enabled: bool,
    marker_builder: MarkerBuilder,
    marker_sync: MarkerSync,
    // true while the markers should be in the world
    markers_shown: bool,
    // path revision, selection, FOV and aspect of the last marker build
    built_revision: u64,
    built_selection: Option<usize>,
    built_fov: f32,
    built_aspect: f32,
    marker_in_world: bool,
    // the handler capture the sent slots were delivered through, 0 before the first capture
    marker_generation: u32,
    // the most frames playback waits in its settle for the markers to leave the world
    marker_settle_limit: u32,
    // frames playback has run since it started
    playback_frames: u32,
    // false when this frame's marker batch could not be delivered and stays pending
    batch_ready: bool,
    screenshot: ScreenshotSequence,
    // a screenshot source exists, so the bar's Screenshot button takes clicks
    can_screenshot: bool,
    toast: Toast,
}

impl FreecamController {
    pub fn new(config: FreecamConfig) -> Self {
        let FreecamConfig { keys, motion, exit_wait_frames, telemetry, markers, look_at, screenshot: _ } =
            config.sanitized();
        let settle_limit = Self::settle_limit(exit_wait_frames, markers.max_shapes, markers.max_per_send);
        Self {
            markers_enabled: markers.enabled,
            marker_sync: MarkerSync::new(markers.max_shapes, markers.max_per_send),
            marker_settle_limit: settle_limit,
            marker_builder: MarkerBuilder::new(markers),
            markers_shown: false,
            built_revision: 0,
            built_selection: None,
            built_fov: 0.0,
            built_aspect: 0.0,
            marker_in_world: false,
            marker_generation: 0,
            playback_frames: 0,
            batch_ready: false,
            editor: BarEditor::new(look_at),
            screenshot: ScreenshotSequence::new(settle_limit),
            can_screenshot: true,
            toast: Toast::new(),
            legend: UiLegend::new(&keys),
            // the hand-off starts at revision 0, so the first publish copies the configured legend
            legend_revision: 1,
            path: PathSession::new(motion.pitch_limit, exit_wait_frames),
            motion: CameraMotion::new(motion),
            input: InputTracker::new(),
            telemetry: SmoothnessTelemetry::new(telemetry),
            exit_wait_frames,
            state: FreecamState::Off,
            menu_open: false,
            toggle_requested: false,
            start_requested: false,
            handover_requested: false,
            lease_lost: false,
            disabled: false,
            play_requested: false,
            last_time: None,
            health: 0,
            dimension: 0,
            cursor_requested: false,
            cursor_mode: false,
            drag: LookDrag::new(),
            pointer_over_ui: false,
            bar_resume: false,
            last_report: None,
        }
    }

    // one event from the mod manager; a lease or enable change takes effect in the next frame
    pub fn on_link_event(&mut self, event: &ModEvent) {
        match *event {
            ModEvent::Keybind { action, phase } => {
                if let Some(action) = FreecamAction::from_index(action) {
                    self.on_keybind(action, phase);
                }
            }
            ModEvent::PointerDelta { dx, dy } => {
                // mouse-look runs outside bar mode, and in bar mode during a look drag
                let looking = !self.cursor_mode || self.drag.is_active();
                self.input.on_pointer_delta(dx, dy, self.is_active() && looking, self.menu_open);
            }
            ModEvent::Wheel { delta } => {
                // in bar mode a wheel notch over the UI goes to the UI and leaves the speed alone
                if !(self.cursor_mode && self.pointer_over_ui) {
                    self.input.on_wheel(delta, self.is_active(), self.menu_open);
                }
            }
            ModEvent::PointerButton { button, down, .. } => {
                let can_drag =
                    self.cursor_mode && self.state == FreecamState::On && !self.menu_open && !self.pointer_over_ui;
                self.drag.on_button(button, down, can_drag);
            }
            ModEvent::Suspended { capability, .. } | ModEvent::Revoked { capability, .. }
                if matches!(capability, Capability::Camera | Capability::Input) =>
            {
                self.lease_lost = true;
                self.input.clear_held();
            }
            ModEvent::Disabled => {
                self.disabled = true;
                self.input.clear_held();
            }
            _ => {}
        }
    }

    // a flight keybind is held from Pressed to Released; every other action runs once on Pressed
    fn on_keybind(&mut self, action: FreecamAction, phase: KeybindPhase) {
        match phase {
            KeybindPhase::Repeat => return,
            KeybindPhase::Released => {
                self.input.on_flight_key(action, false);
                return;
            }
            KeybindPhase::Pressed => self.input.on_flight_key(action, true),
        }
        if self.menu_open {
            return;
        }
        let active = self.is_active();
        match action {
            FreecamAction::Toggle if active => self.toggle_requested = true,
            FreecamAction::Toggle => self.start_requested = true,
            FreecamAction::ToggleSmooth if active => {
                let smooth = self.motion.toggle_smooth();
                curia::info!("freecam smooth camera", { on: smooth });
            }
            FreecamAction::AddPoint if self.state == FreecamState::On => {
                self.editor.begin_key(KeyEdit::Insert, self.motion.pose(), &mut self.path);
            }
            FreecamAction::DeletePoint if self.state == FreecamState::On => self.editor.delete_selected(&mut self.path),
            FreecamAction::ToggleBar if active => self.cursor_requested = true,
            _ => {}
        }
    }

    // true once after a toggle while off: the mod layer asks the mod manager for the flight leases and answers
    // with finish_start
    pub fn take_start_request(&mut self) -> bool {
        std::mem::take(&mut self.start_requested)
    }

    // the mod manager's answer to the flight leases; granted leases start freecam in the next frame
    pub fn finish_start(&mut self, leases: StartLeases) {
        match leases {
            StartLeases::Granted => self.toggle_requested = true,
            StartLeases::Denied(capability) => {
                let refusal = StartRefusal::LeaseDenied(capability);
                curia::warn!("freecam start refused", { refusal: format!("{refusal:?}") });
            }
        }
    }

    // a changed config.hcl while freecam may run: the keys, motion and look-at take effect at once; the rest waits
    // for a restart
    pub fn apply_config(&mut self, config: FreecamConfig) {
        let config = config.sanitized();
        self.legend = UiLegend::new(&config.keys);
        self.legend_revision = self.legend_revision.wrapping_add(1);
        self.path.set_pitch_limit(config.motion.pitch_limit);
        self.motion.set_config(config.motion);
        self.editor.set_look_at(config.look_at);
    }

    // whether the UI's last frame had the pointer over a panel or in use; a right press there goes to the UI and
    // starts no look drag
    pub fn set_pointer_over_ui(&mut self, over: bool) {
        self.pointer_over_ui = over;
    }

    // the answer to the frame's ray request: the hit position, or None when the ray hit nothing; an
    // answer with no request waiting changes nothing
    pub fn on_ray_result(&mut self, hit: Option<Vec3>) {
        self.editor.on_ray_result(hit, &mut self.path, &mut self.telemetry);
    }

    // a flight key went down at `time_seconds`, for the key-to-move telemetry
    pub fn on_key_probe(&mut self, time_seconds: f64) {
        self.telemetry.record_key_probe(time_seconds);
    }

    // from_other_mod is true while another mod holds the Camera lease; its camera instructions are not the server's
    pub fn on_server_instruction(&mut self, kind: ServerInstruction, from_other_mod: bool) {
        if kind == ServerInstruction::SetOrClear && self.is_active() && !from_other_mod {
            self.handover_requested = true;
        }
    }

    pub fn state(&self) -> FreecamState {
        self.state
    }

    pub fn path(&self) -> &PathSession {
        &self.path
    }

    pub fn ui_snapshot(&self) -> UiSnapshot {
        let count = self.path.keys().len();
        let curve = self.path.curve();
        let mut point_distances = [0.0; PATH_CAPACITY];
        for (index, slot) in point_distances.iter_mut().take(count).enumerate() {
            *slot = curve.point_distance(index);
        }
        let speed = self.motion.speed();
        let playhead = self.editor.playhead();
        UiSnapshot {
            visible: self.ui_visible() && !self.screenshot.hides(),
            cursor_mode: self.cursor_mode,
            can_screenshot: self.can_screenshot,
            point_count: count as u32,
            point_distances,
            playhead,
            length: curve.length(),
            playhead_seconds: self.path.seconds_at(playhead, speed),
            total_seconds: self.path.shot_seconds(speed),
            speed,
            selection: self.editor.selection().filter(|selected| *selected < count),
            playing: self.state == FreecamState::Playing,
            clear_armed: self.editor.clear_armed(),
            toast: self.toast.kind(),
            toast_file: self.toast.file(),
            toast_file_len: self.toast.file_len(),
        }
    }

    // whether a screenshot source exists; the mod layer sets it from what the mod layer or the mod manager can do
    pub fn set_can_screenshot(&mut self, can_screenshot: bool) {
        self.can_screenshot = can_screenshot;
    }

    pub fn legend(&self) -> &UiLegend {
        &self.legend
    }

    // changes each time the legend is built again, so a copy of it knows when it is stale
    pub fn legend_revision(&self) -> u64 {
        self.legend_revision
    }

    // bar clicks act only in bar mode while freecam edits
    pub fn apply_ui_action(&mut self, action: UiAction) {
        if !self.cursor_mode || self.state != FreecamState::On {
            return;
        }
        if matches!(action, UiAction::Screenshot) {
            self.begin_screenshot();
        } else {
            self.play_requested |= self.editor.apply(action, &mut self.path, &mut self.motion);
        }
    }

    // starts the hide, wait and capture sequence; one shot at a time
    fn begin_screenshot(&mut self) {
        if self.screenshot.request() {
            curia::info!("freecam screenshot started");
        } else {
            curia::info!("freecam screenshot refused: one is already running");
        }
    }

    // the frame of the capture request was copied (`ok`) or could not be; the drawing comes back. A failure shows
    // the Failed toast; a copied frame waits for its written answer. An answer with no request waiting changes nothing
    pub fn on_capture_taken(&mut self, sequence: u32, ok: bool) {
        if !self.screenshot.taken(sequence, ok) {
            return;
        }
        if !ok {
            self.toast.show(ToastKind::Failed, "");
        }
    }

    // the PNG of a taken shot was written to `file_name` (`ok`) or failed; shows the Saved or the Failed toast. An
    // answer with no taken shot waiting for it changes nothing
    pub fn on_capture_written(&mut self, sequence: u32, ok: bool, file_name: &str) {
        if !self.screenshot.written(sequence) {
            return;
        }
        if ok {
            self.toast.show(ToastKind::Saved, file_name);
        } else {
            self.toast.show(ToastKind::Failed, "");
        }
    }

    // the UI frames measured since the last hand-off
    pub fn record_ui_summary(&mut self, summary: &UiTimingSummary) {
        self.telemetry.record_ui_summary(summary);
    }

    // the newest telemetry window, None until telemetry closed one
    pub fn last_telemetry_report(&self) -> Option<TelemetryReport> {
        self.last_report
    }

    pub fn frame(&mut self, status: &FrameStatus) -> FrameOutput {
        // the bar key while a path plays stops the playback instead of opening the cursor
        let stop_playback = self.cursor_requested && self.state == FreecamState::Playing;
        self.play_requested |= stop_playback;
        let toggle_cursor = std::mem::take(&mut self.cursor_requested) && !stop_playback;
        let dt = self.advance_clock(status.time_seconds);
        let mut output = self.advance(status, dt);
        self.editor.clamp(&self.path);
        let resume = self.take_bar_resume(status);
        self.update_cursor(status, toggle_cursor || resume);
        self.drag.update(self.cursor_mode);
        if !self.cursor_mode {
            self.editor.disarm();
        }
        output.cursor_mode = self.cursor_mode && !self.drag.is_active();
        output.leases_wanted = self.is_active();
        output.ui_visible = self.ui_visible();
        output.camera_owned = self.is_active();
        output.ray = self.editor.update_pending(self.state == FreecamState::On, &mut self.path, &mut self.telemetry);
        if self.state != FreecamState::On {
            self.screenshot.abort();
        }
        let marker_us = self.update_markers(status);
        self.telemetry.record_path_rebuild(self.path.take_rebuild_us() + marker_us);
        self.update_screenshot(status, dt, &mut output);
        output
    }

    // bar mode returns once the playback it started is over and the UI can show; dropped when
    // freecam leaves editing
    fn take_bar_resume(&mut self, status: &FrameStatus) -> bool {
        match self.state {
            FreecamState::Playing => false,
            FreecamState::On if self.bar_resume => {
                if self.cursor_mode {
                    self.bar_resume = false;
                    return false;
                }
                let ready = self.ui_visible() && status.focused;
                self.bar_resume = !ready;
                ready
            }
            _ => {
                self.bar_resume = false;
                false
            }
        }
    }

    // the marker changes for the mod layer to deliver this frame; valid until the next `frame`
    pub fn marker_batch(&self) -> &[MarkerEntry] {
        if self.batch_ready { self.marker_sync.batch() } else { &[] }
    }

    // the most entries `marker_batch` or `marker_shutdown_batch` can return
    pub fn marker_capacity(&self) -> usize {
        self.marker_sync.capacity()
    }

    // a remove for every live marker, for the mod layer to deliver before shutdown; valid until the next
    // call on the controller. Markers stay off after it, so a later frame builds none
    pub fn marker_shutdown_batch(&mut self) -> &[MarkerEntry] {
        self.markers_enabled = false;
        self.batch_ready = false;
        self.markers_shown = false;
        self.marker_sync.shutdown_batch()
    }

    // markers show while freecam edits; they are rebuilt when the path, the selection, the view size or the world
    // changes, and removed when freecam leaves editing; returns the microseconds a rebuild took, 0 without one
    fn update_markers(&mut self, status: &FrameStatus) -> f32 {
        let recaptured = status.handler_generation != 0 && status.handler_generation != self.marker_generation;
        if recaptured {
            self.marker_generation = status.handler_generation;
        }
        let rejoined = (status.in_world && !self.marker_in_world) || recaptured;
        if (self.marker_in_world && !status.in_world) || recaptured {
            self.marker_sync.forget_sent();
        }
        self.marker_in_world = status.in_world;
        if !self.markers_enabled {
            return 0.0;
        }
        let show = self.state == FreecamState::On && !self.screenshot.hides();
        let mut rebuild_us = 0.0;
        if show {
            let revision = self.path.revision();
            let stale = revision != self.built_revision
                || self.editor.selection() != self.built_selection
                || self.view_changed(status);
            if !self.markers_shown || rejoined || stale {
                let started = Instant::now();
                self.rebuild_markers(status, revision);
                rebuild_us = started.elapsed().as_secs_f32() * 1e6;
            }
        } else if self.markers_shown {
            self.marker_sync.hide();
        }
        self.markers_shown = show;
        self.batch_ready = status.in_world && status.ready && status.handler_generation != 0;
        let sent = if self.batch_ready { self.marker_sync.next_batch().len() } else { 0 };
        let live = self.marker_sync.live_slots();
        let reductions = self.marker_builder.reductions();
        self.telemetry.record_markers(live, sent, status.last_shape_send_us, reductions);
        rebuild_us
    }

    // advances the screenshot sequence once the marker sync is known for this frame
    fn update_screenshot(&mut self, status: &FrameStatus, dt: f64, output: &mut FrameOutput) {
        let live = self.marker_sync.live_slots();
        match self.screenshot.step(live, status.present_count, dt) {
            ScreenshotStep::Capture(request) => output.capture = Some(request),
            ScreenshotStep::TimedOut => {
                curia::warn!("freecam screenshot dropped: no capture result in time");
                self.toast.show(ToastKind::Failed, "");
            }
            ScreenshotStep::MarkersStuck => {
                curia::warn!("freecam screenshot skipped: markers could not leave the world in time", { live: live });
                self.toast.show(ToastKind::Failed, "");
            }
            ScreenshotStep::WriteTimedOut => {
                curia::warn!("freecam screenshot failed: the file was not written in time");
                self.toast.show(ToastKind::Failed, "");
            }
            ScreenshotStep::Waiting => {}
        }
        output.hide_ui = self.screenshot.hides();
    }

    fn rebuild_markers(&mut self, status: &FrameStatus, revision: u64) {
        let keys = self.path.keys();
        let selection = self.editor.selection();
        let shapes = self.marker_builder.build(keys, self.path.curve(), selection, status.fov_y_degrees, status.aspect);
        self.marker_sync.set_target(shapes, status.dimension);
        self.built_revision = revision;
        self.built_selection = selection;
        self.built_fov = status.fov_y_degrees;
        self.built_aspect = status.aspect;
    }

    // the exit wait plus the frames a full spread of removes needs
    fn settle_limit(exit_wait_frames: u32, max_shapes: usize, max_per_send: usize) -> u32 {
        let spread = max_shapes.div_ceil(max_per_send.max(1));
        exit_wait_frames.saturating_add(u32::try_from(spread).unwrap_or(u32::MAX))
    }

    fn view_changed(&self, status: &FrameStatus) -> bool {
        Self::moved(self.built_fov, status.fov_y_degrees) || Self::moved(self.built_aspect, status.aspect)
    }

    // true when the value changed by more than `VIEW_CHANGE` of the old value, or became or stopped being finite
    fn moved(old: f32, new: f32) -> bool {
        if !old.is_finite() || !new.is_finite() {
            return old.is_finite() != new.is_finite();
        }
        (new - old).abs() > old.abs() * VIEW_CHANGE
    }

    // the UI shows while freecam edits and no menu is open
    fn ui_visible(&self) -> bool {
        self.state == FreecamState::On && !self.menu_open
    }

    // cursor mode needs a visible UI and a focused window; it ends the moment either goes away, and the look and
    // wheel input gathered while it was on is dropped
    fn update_cursor(&mut self, status: &FrameStatus, toggle: bool) {
        let allowed = self.ui_visible() && status.focused;
        let next = allowed && (self.cursor_mode != toggle);
        let was = std::mem::replace(&mut self.cursor_mode, next);
        if was && !next {
            self.input.take_look();
            self.input.take_scroll();
        }
    }

    fn advance(&mut self, status: &FrameStatus, dt: f64) -> FrameOutput {
        self.editor.tick(dt);
        self.toast.tick(dt);
        self.menu_open = status.menu_open;
        self.path.observe_world(status.in_world, status.dimension);
        if !status.focused {
            self.input.clear_held();
        }
        let toggle = std::mem::take(&mut self.toggle_requested);
        let handover = std::mem::take(&mut self.handover_requested);
        let play = std::mem::take(&mut self.play_requested);
        let lease_lost = std::mem::take(&mut self.lease_lost);
        let disabled = std::mem::take(&mut self.disabled);
        if self.is_active() {
            if !status.in_world {
                return self.finish("left the world");
            }
            if handover {
                return self.finish("the server took the camera");
            }
            if disabled {
                let mut out = self.finish("the mod manager disabled freecam");
                out.camera = CameraCommand::Clear;
                return out;
            }
            if lease_lost && matches!(self.state, FreecamState::On | FreecamState::Playing) {
                return self.begin_exit("another mod took the camera or the input");
            }
        }
        match self.state {
            FreecamState::Off if toggle => self.try_start(status),
            FreecamState::Off => FrameOutput::default(),
            FreecamState::On => self.run(status, dt, toggle, play),
            FreecamState::Playing => self.play(status, dt, toggle, play),
            FreecamState::Exiting { frames_left } => self.count_down(frames_left, status.camera_held_by_other),
        }
    }

    fn is_active(&self) -> bool {
        self.state != FreecamState::Off
    }

    // seconds since the previous frame, 0 on the first, never clamped
    fn advance_clock(&mut self, time_seconds: f64) -> f64 {
        let dt = self.last_time.map_or(0.0, |last| (time_seconds - last).max(0.0));
        self.last_time = Some(time_seconds);
        dt
    }

    fn refusal(status: &FrameStatus) -> Option<StartRefusal> {
        if !status.in_world {
            Some(StartRefusal::NotInWorld)
        } else if status.menu_open {
            Some(StartRefusal::MenuOpen)
        } else if !status.ready {
            Some(StartRefusal::NotReady)
        } else if !status.player_camera_active {
            Some(StartRefusal::ServerCameraActive)
        } else {
            None
        }
    }

    fn try_start(&mut self, status: &FrameStatus) -> FrameOutput {
        if let Some(refusal) = Self::refusal(status) {
            curia::info!("freecam start refused", { refusal: format!("{refusal:?}") });
            return FrameOutput::default();
        }
        self.input.take_look();
        self.input.take_scroll();
        self.motion.reset(status.start_pose);
        self.telemetry.restart();
        self.health = status.health;
        self.dimension = status.dimension;
        self.state = FreecamState::On;
        curia::info!("freecam on");
        Self::active_output(status.start_pose)
    }

    fn run(&mut self, status: &FrameStatus, dt: f64, toggle: bool, play: bool) -> FrameOutput {
        if let Some(output) = self.check_exit(status, toggle) {
            return output;
        }
        if play && !status.menu_open {
            let from = self.editor.prepare_play(&mut self.path);
            if self.path.start(self.motion.speed(), from) {
                self.state = FreecamState::Playing;
                self.bar_resume = self.cursor_mode;
                self.playback_frames = 0;
                curia::info!("freecam playing the path");
                return self.play_frame(status, dt);
            }
        }
        let dt = dt as f32;
        let pose = if status.menu_open { self.hold_pose() } else { self.step_pose(dt) };
        let output = Self::active_output(pose);
        self.record_telemetry(status, dt, &output, self.motion.velocity());
        output
    }

    // movement and look input are drained and discarded while the path plays
    fn play(&mut self, status: &FrameStatus, dt: f64, toggle: bool, stop: bool) -> FrameOutput {
        if let Some(output) = self.check_exit(status, toggle) {
            return output;
        }
        self.input.take_look();
        self.input.take_scroll();
        if stop {
            return self.end_playback("stopped");
        }
        self.play_frame(status, dt)
    }

    fn play_frame(&mut self, status: &FrameStatus, dt: f64) -> FrameOutput {
        let before_pose = self.motion.pose();
        let before = before_pose.position;
        // the shot starts once every marker has left the world, or when the markers cannot drain in time
        self.playback_frames = self.playback_frames.saturating_add(1);
        if self.marker_sync.live_slots() > 0 && self.playback_frames <= self.marker_settle_limit {
            self.path.hold_settle();
        }
        let (pose, step) = self.path.step(dt, status.menu_open);
        self.motion.reset(pose);
        self.editor.set_playhead(step.distance);
        if step.finished {
            return self.end_playback("path end");
        }
        let output = Self::active_output(pose);
        if !step.settling {
            let dt = dt as f32;
            let velocity = if dt > 0.0 { (pose.position - before) * (1.0 / dt) } else { Vec3::default() };
            self.telemetry.record_aim_turn(before_pose.forward(), pose.forward());
            self.record_telemetry(status, dt, &output, velocity);
        }
        output
    }

    // the camera stands still where playback left it and freecam control resumes
    fn end_playback(&mut self, reason: &str) -> FrameOutput {
        self.path.stop();
        self.state = FreecamState::On;
        curia::info!("freecam playback over", { reason: reason });
        Self::active_output(self.motion.pose())
    }

    // the exits shared by editing and playback
    fn check_exit(&mut self, status: &FrameStatus, toggle: bool) -> Option<FrameOutput> {
        let damaged = status.health < self.health;
        self.health = status.health;
        if toggle {
            return Some(self.begin_exit("toggle"));
        }
        if damaged {
            return Some(self.begin_exit("damage"));
        }
        if status.dimension != self.dimension {
            return Some(self.begin_exit("dimension change"));
        }
        None
    }

    fn record_telemetry(&mut self, status: &FrameStatus, dt: f32, output: &FrameOutput, velocity: Vec3) {
        let sent_set = matches!(output.camera, CameraCommand::Set(_));
        let forward = output.render_override.map_or(Vec3::default(), |pose| pose.forward());
        self.telemetry.record(dt, status, sent_set, velocity, forward);
        if let Some(report) = self.telemetry.take_report() {
            Self::log_report(&report);
            self.last_report = Some(report);
        }
    }

    fn log_report(report: &TelemetryReport) {
        curia::info!("freecam telemetry", {
                fps: report.fps,
                min_dt: report.min_dt,
                max_dt: report.max_dt,
                step_ratio_min: report.step_ratio_min,
                step_ratio_max: report.step_ratio_max,
                aim_turn_max_deg: report.aim_turn_max_deg,
                sets_per_second: report.sets_per_second,
                camera_updates_per_second: report.camera_updates_per_second,
                max_drift_degrees: report.max_drift_degrees,
                missed_overrides: report.missed_overrides,
                ui_cpu_ms_avg: report.ui_cpu_ms_avg,
                ui_cpu_ms_max: report.ui_cpu_ms_max,
                ui_gpu_ms_avg: report.ui_gpu_ms_avg,
                ui_gpu_ms_max: report.ui_gpu_ms_max,
                ui_vertices: report.ui_vertices,
                ui_draws: report.ui_draws,
                ui_dropped_events: report.ui_dropped_events,
                ui_skipped_frames: report.ui_skipped_frames,
                marker_live_slots: report.marker_live_slots,
                marker_entries_sent: report.marker_entries_sent,
                marker_max_send_entries: report.marker_max_send_entries,
                marker_max_send_us: report.marker_max_send_us,
                marker_reductions: report.marker_reductions,
                path_rebuild_us_max: report.path_rebuild_us_max,
                look_at_rays_answered: report.look_at_rays_answered,
                look_at_rays_timed_out: report.look_at_rays_timed_out,
                key_to_move_ms_p50: report.key_to_move_ms_p50,
                key_to_move_ms_p99: report.key_to_move_ms_p99,
                key_to_move_ms_max: report.key_to_move_ms_max,
                key_to_move_samples: report.key_to_move_samples,
        });
    }

    fn hold_pose(&mut self) -> CameraPose {
        self.motion.hold();
        self.input.take_look();
        self.input.take_scroll();
        self.motion.pose()
    }

    fn step_pose(&mut self, dt: f32) -> CameraPose {
        let (dx, dy) = self.input.take_look();
        self.motion.add_look(dx, dy);
        self.motion.scroll(self.input.take_scroll());
        self.motion.step(dt, self.input.held())
    }

    fn active_output(pose: CameraPose) -> FrameOutput {
        FrameOutput { camera: CameraCommand::Set(pose), render_override: Some(pose), ..FrameOutput::default() }
    }

    // the frame that starts the exit counts as exit frame 1
    fn begin_exit(&mut self, reason: &str) -> FrameOutput {
        self.path.stop();
        let frames_left = self.exit_wait_frames.saturating_sub(1);
        self.state = FreecamState::Exiting { frames_left };
        curia::info!("freecam exiting", { reason: reason });
        Self::waiting_output()
    }

    // the last exit frame gives the player camera back with a Clear, unless another mod now drives the camera: a Clear
    // would remove that mod's camera too
    fn count_down(&mut self, frames_left: u32, camera_held_by_other: bool) -> FrameOutput {
        if frames_left > 1 {
            self.state = FreecamState::Exiting { frames_left: frames_left - 1 };
            return Self::waiting_output();
        }
        let mut out = self.finish("exit wait over");
        if !camera_held_by_other {
            out.camera = CameraCommand::Clear;
        }
        out
    }

    fn waiting_output() -> FrameOutput {
        FrameOutput::default()
    }

    // switches off and sends no camera command
    fn finish(&mut self, reason: &str) -> FrameOutput {
        self.path.stop();
        self.state = FreecamState::Off;
        curia::info!("freecam off", { reason: reason });
        FrameOutput::default()
    }
}
