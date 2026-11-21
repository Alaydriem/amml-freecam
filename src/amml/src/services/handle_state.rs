use amm_core::structs::event::ModEvent;
use amml_api::Logger;
use freecam_core::config::{FreecamConfig, KeyConfig};
use freecam_core::services::FreecamController;
use freecam_core::structs::{FrameOutput, FrameStatus, FreecamState, MarkerEntry, ServerInstruction, Vec3};

use crate::services::{LinkSession, ScreenshotPath, ScreenshotRelay};
use crate::types::InputItem;

// everything behind the session's lock: the controller, the manager link and the screenshot relay. Game thread.
//
// Bound: the marker buffer holds at most the controller's marker capacity (`markers.max_shapes`), reserved once in
// `new()`. A fill never grows past it, so the buffer is never reallocated
pub struct HandleState {
    controller: FreecamController,
    link: LinkSession,
    relay: ScreenshotRelay,
    markers: Vec<MarkerEntry>,
    linked: bool,
    can_screenshot: bool,
    log: Logger,
}

impl HandleState {
    pub fn new(link: LinkSession, log: Logger, can_screenshot: bool) -> Self {
        let config = FreecamConfig::default();
        let relay = ScreenshotRelay::new(Self::screenshot_path(&config));
        let mut controller = FreecamController::new(config);
        controller.set_can_screenshot(can_screenshot);
        let capacity = controller.marker_capacity();
        Self { controller, link, relay, markers: Vec::with_capacity(capacity), linked: false, can_screenshot, log }
    }

    pub fn controller_mut(&mut self) -> &mut FreecamController {
        &mut self.controller
    }

    // the first frame with the manager connected reads config.hcl and the language, and builds the controller from
    // that config (freecam is off before it: no bind can reach it without the manager); the config, for the keys
    // and the telemetry switch
    pub fn connect_if_ready(&mut self) -> Option<FreecamConfig> {
        if self.linked || !self.link.connected() {
            return None;
        }
        self.linked = true;
        self.link.apply_locale();
        let config = self.link.config();
        let can_screenshot = self.can_screenshot;
        self.relay.set_path(Self::screenshot_path(&config));
        self.controller = FreecamController::new(config.clone());
        self.controller.set_can_screenshot(can_screenshot);
        self.markers = Vec::with_capacity(self.controller.marker_capacity());
        self.log.info("freecam linked", &[]);
        Some(config)
    }

    // the manager's events since the last frame, in order. While freecam holds its Input lease it matches its own
    // binds, so a manager keybind then is dropped (no action runs twice); so is the keybind of an action in `lingering`
    // (bit = action index: a bind freecam let go of while its key stays held). Returns the new keys after a settings
    // change
    pub fn apply_events(&mut self, lingering: u16) -> Option<KeyConfig> {
        let holding = self.link.holding();
        let Self { controller, link, relay, log, .. } = self;
        let mut config_changed = false;
        let mut locale_changed = false;
        let mut disabled = false;
        for event in link.drain() {
            if relay.on_event(event, controller, log) {
                continue;
            }
            match event {
                ModEvent::SettingsChanged { .. } => config_changed = true,
                ModEvent::LocaleChanged => locale_changed = true,
                ModEvent::Keybind { .. } if holding => {}
                ModEvent::Keybind { action, .. } if *action < 16 && lingering & (1 << *action) != 0 => {}
                ModEvent::Disabled => {
                    disabled = true;
                    controller.on_link_event(event);
                }
                other => controller.on_link_event(other),
            }
        }
        if disabled {
            self.link.forget_leases();
        }
        if locale_changed {
            self.link.apply_locale();
        }
        config_changed.then(|| {
            let config = self.link.config();
            self.reconfigure(config)
        })
    }

    // the inputs the listeners queued since the last frame
    pub fn apply_inputs(&mut self, inputs: &[InputItem]) {
        for input in inputs {
            match *input {
                InputItem::Bind(action, phase) => self.controller.on_bind(action, phase),
                InputItem::Look { dx, dy } => self.controller.on_look_counts(dx, dy),
                InputItem::Wheel(notches) => self.controller.on_wheel(notches),
                InputItem::Button(button, down) => self.controller.on_button(button, down),
            }
        }
    }

    // a toggle while off asks the manager for the flight leases, and core gets the answer before its frame
    pub fn start_if_asked(&mut self) {
        if self.controller.take_start_request() {
            let leases = self.link.acquire_flight();
            self.controller.finish_start(leases);
        }
    }

    // a capture core asks for goes to the loader, the cursor follows core, and the leases go back once core no longer
    // wants them
    pub fn apply_output(&mut self, output: &FrameOutput) {
        if let Some(capture) = output.capture {
            self.relay.request(capture.sequence, &self.link, &mut self.controller);
        }
        if !output.leases_wanted {
            if self.link.holding() {
                self.link.release_flight();
            }
            return;
        }
        self.link.sync_cursor(output.cursor_mode);
    }

    // the status as core needs it: while freecam exits, whether another mod now holds the Camera lease
    pub fn complete_status(&self, status: &mut FrameStatus) {
        status.camera_held_by_other =
            matches!(self.controller.state(), FreecamState::Exiting { .. }) && self.link.camera_held_by_other();
    }

    // a camera instruction reached the client: `from_other_mod` when a mod's free camera call sent it
    pub fn server_instruction(&mut self, kind: ServerInstruction, from_other_mod: bool) {
        self.controller.on_server_instruction(kind, from_other_mod);
    }

    pub fn ray_result(&mut self, hit: Option<Vec3>) {
        self.controller.on_ray_result(hit);
    }

    // copies the frame's marker batch into the reused buffer
    pub fn publish_markers(&mut self) -> &[MarkerEntry] {
        self.markers.clear();
        let room = self.markers.capacity();
        self.markers.extend(self.controller.marker_batch().iter().take(room).copied());
        &self.markers
    }

    pub fn holding(&self) -> bool {
        self.link.holding()
    }

    pub fn panel_open(&self) -> bool {
        self.link.panel_open()
    }

    fn reconfigure(&mut self, config: FreecamConfig) -> KeyConfig {
        self.relay.set_path(Self::screenshot_path(&config));
        let keys = config.keys.clone().sanitized();
        self.controller.apply_config(config);
        keys
    }

    fn screenshot_path(config: &FreecamConfig) -> Option<ScreenshotPath> {
        let folder = config.screenshot.clone().sanitized().folder;
        ScreenshotPath::new(&folder, |name| std::env::var(name).ok())
    }
}
