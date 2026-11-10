use std::path::Path;
use std::sync::Arc;

use amm_abi::types::AmmStatus;
use amm_client::link::{EventDrain, ManagerLink};
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::lease::AcquireOutcome;
use amml_i18n::localize::Localizer;
use freecam_core::config::FreecamConfig;
use freecam_core::structs::StartLeases;
use freecam_ui::runtime::FreecamLocale;

// freecam's link to the mod manager, on the game thread only: the frame's events, the flight leases and the cursor.
//
// Bound: `events` holds at most DRAIN_LIMIT events, reserved in `new()`; the events past it wait in the manager's
// queue for the next frame
pub struct LinkSession {
    link: ManagerLink,
    events: Vec<ModEvent>,
    holding: bool,
    cursor_on: bool,
    // the manager has the screenshot call (ABI 1.2), until a request says it does not
    can_screenshot: bool,
}

impl LinkSession {
    // the manager's own queue holds 256 events per mod
    pub const DRAIN_LIMIT: usize = 4 * EventDrain::CAPACITY;
    // the leases of a flight, in the order they are asked for
    pub const FLIGHT: [Capability; 5] =
        [Capability::Camera, Capability::Input, Capability::Hud, Capability::Overlay, Capability::WorldDraw];

    pub fn new(link: ManagerLink) -> Self {
        let can_screenshot = link.supports_screenshots();
        Self { link, events: Vec::with_capacity(Self::DRAIN_LIMIT), holding: false, cursor_on: false, can_screenshot }
    }

    // reads the language the person picked in the manager and gives the frame's layers freecam's words in it; a
    // manager older than ABI 1.5 has no language setting, so freecam shows English
    pub fn apply_locale(&self) {
        let code = self.link.locale().unwrap_or_default();
        self.link.set_localizer(FreecamLocale::localizer(&code));
    }

    // the localizer the frame's layers draw freecam's words with now
    pub fn localizer(&self) -> Arc<Localizer> {
        self.link.localizer()
    }

    // the events queued since the last frame, up to DRAIN_LIMIT
    pub fn drain(&mut self) -> &[ModEvent] {
        self.events.clear();
        while self.events.len() < Self::DRAIN_LIMIT {
            let before = self.events.len();
            let room = Self::DRAIN_LIMIT - before;
            self.events.extend(self.link.events().take(room));
            if self.events.len() - before < EventDrain::CAPACITY {
                break;
            }
        }
        &self.events
    }

    // asks for every flight lease; on the first denial gives back the ones already granted
    pub fn acquire_flight(&mut self) -> StartLeases {
        for (index, capability) in Self::FLIGHT.iter().enumerate() {
            let denied = match self.link.acquire(*capability) {
                Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => false,
                Ok(AcquireOutcome::Denied { .. }) => true,
                Err(error) => {
                    curia::warn!("flight lease not granted", { capability: format!("{capability:?}"), error: error.to_string() });
                    true
                }
            };
            if denied {
                for granted in &Self::FLIGHT[..index] {
                    self.link.release(*granted);
                }
                return StartLeases::Denied(*capability);
            }
        }
        self.holding = true;
        StartLeases::Granted
    }

    // gives back every flight lease and turns the cursor off
    pub fn release_flight(&mut self) {
        self.sync_cursor(false);
        for capability in Self::FLIGHT {
            self.link.release(capability);
        }
        self.holding = false;
    }

    pub fn holding(&self) -> bool {
        self.holding
    }

    // the manager forgets every lease of a disabled mod
    pub fn forget_leases(&mut self) {
        self.holding = false;
        self.cursor_on = false;
    }

    // tells the manager the cursor mode when it changed; true when it was sent
    pub fn sync_cursor(&mut self, on: bool) -> bool {
        if on == self.cursor_on {
            return false;
        }
        self.cursor_on = on;
        if let Err(error) = self.link.set_cursor_mode(on) {
            curia::warn!("cursor mode not set", { on: on, error: error.to_string() });
        }
        true
    }

    // the checked config.hcl, or the defaults when it cannot be read
    pub fn config(&self) -> FreecamConfig {
        match self.link.config::<FreecamConfig>() {
            Ok(config) => config.sanitized(),
            Err(error) => {
                curia::warn!("config.hcl not read; using the defaults", { error: error.to_string() });
                FreecamConfig::default()
            }
        }
    }

    pub fn can_screenshot(&self) -> bool {
        self.can_screenshot
    }

    // asks the manager for a PNG of the next Present at `path`; the manager's sequence, or None when it refused.
    // An Unsupported answer means this manager has no screenshots, and the button goes off
    pub fn request_screenshot(&mut self, path: &Path) -> Option<u32> {
        match self.link.request_screenshot(path) {
            Ok(sequence) => Some(sequence),
            Err(error) => {
                if error == LinkError::from(AmmStatus::Unsupported) {
                    self.can_screenshot = false;
                }
                curia::warn!("screenshot refused by the mod manager", { error: error.to_string() });
                None
            }
        }
    }

    // another mod holds the Camera lease, so a camera instruction now is not the server's
    pub fn camera_held_by_other(&self) -> bool {
        self.link.held_by_other(Capability::Camera)
    }
}
