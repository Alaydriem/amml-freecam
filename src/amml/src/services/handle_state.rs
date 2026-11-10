use amm_core::structs::event::ModEvent;
use freecam_core::services::FreecamController;
use freecam_core::structs::{FrameOutput, FrameStatus, FreecamState, MarkerEntry, ServerInstruction};

use crate::services::{LinkSession, ScreenshotRelay};

// everything behind the session's lock
//
// Bound: each marker buffer holds at most the controller's marker capacity (`markers.max_shapes`), reserved once in
// `new()`. A fill never grows past it, so the buffer is never reallocated.
pub struct HandleState {
    controller: FreecamController,
    session: LinkSession,
    relay: ScreenshotRelay,
    // the last fc_frame marker batch
    markers: Vec<MarkerEntry>,
    // the last fc_markers_shutdown batch
    shutdown_markers: Vec<MarkerEntry>,
}

impl HandleState {
    pub fn new(controller: FreecamController, session: LinkSession, relay: ScreenshotRelay) -> Self {
        let capacity = controller.marker_capacity();
        Self {
            controller,
            session,
            relay,
            markers: Vec::with_capacity(capacity),
            shutdown_markers: Vec::with_capacity(capacity),
        }
    }

    pub fn controller_mut(&mut self) -> &mut FreecamController {
        &mut self.controller
    }

    // the manager's events since the last frame, in order: a settings change reads config.hcl again, a language
    // change reads the language again, every other event goes to core
    pub fn apply_events(&mut self) {
        let Self { controller, session, relay, .. } = self;
        let mut config_changed = false;
        let mut locale_changed = false;
        let mut disabled = false;
        for event in session.drain() {
            if relay.on_event(event, controller) {
                continue;
            }
            match event {
                ModEvent::SettingsChanged { .. } => config_changed = true,
                ModEvent::LocaleChanged => locale_changed = true,
                ModEvent::Disabled => {
                    disabled = true;
                    controller.on_link_event(event);
                }
                other => controller.on_link_event(other),
            }
        }
        if disabled {
            session.forget_leases();
        }
        if config_changed {
            controller.apply_config(session.config());
        }
        if locale_changed {
            session.apply_locale();
        }
    }

    // a toggle while off asks the manager for the flight leases, and core gets the answer before its frame
    pub fn start_if_asked(&mut self) {
        if self.controller.take_start_request() {
            let leases = self.session.acquire_flight();
            self.controller.finish_start(leases);
        }
    }

    // a capture core asks for goes to the manager, the cursor follows core, and the leases go back once core no
    // longer wants them
    pub fn apply_output(&mut self, output: &FrameOutput) {
        if let Some(capture) = output.capture {
            self.relay.request(capture.sequence, &mut self.session, &mut self.controller);
        }
        if !output.leases_wanted {
            if self.session.holding() {
                self.session.release_flight();
            }
            return;
        }
        self.session.sync_cursor(output.cursor_mode);
    }

    // the status as core needs it: while freecam exits, whether another mod now holds the Camera lease
    pub fn complete_status(&self, status: &mut FrameStatus) {
        status.camera_held_by_other =
            matches!(self.controller.state(), FreecamState::Exiting { .. }) && self.session.camera_held_by_other();
    }

    pub fn server_instruction(&mut self, kind: ServerInstruction) {
        let from_other_mod = self.session.camera_held_by_other();
        self.controller.on_server_instruction(kind, from_other_mod);
    }

    // copies the frame's marker batch into the reused buffer
    pub fn publish_markers(&mut self) -> &[MarkerEntry] {
        Self::fill(&mut self.markers, self.controller.marker_batch());
        &self.markers
    }

    // a remove for every live marker, copied into its own reused buffer so the last frame batch stays readable
    pub fn shutdown_markers(&mut self) -> &[MarkerEntry] {
        Self::fill(&mut self.shutdown_markers, self.controller.marker_shutdown_batch());
        &self.shutdown_markers
    }

    fn fill(buffer: &mut Vec<MarkerEntry>, batch: &[MarkerEntry]) {
        buffer.clear();
        let room = buffer.capacity();
        buffer.extend(batch.iter().take(room).copied());
    }
}
