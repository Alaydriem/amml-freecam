use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use amm_client::link::ManagerLink;
use amm_core::errors::LinkError;
use amml_api::{Area, Ctx};
use freecam_core::config::KeyConfig;
use freecam_core::structs::{FrameOutput, FrameStatus, MarkerEntry, ServerInstruction, Vec3};

use crate::services::{FreecamDescriptor, FreecamFrameHandler, HandleState, LinkSession, RenderClock, UiHandoff};
use crate::types::{InputItem, SessionFrame};

// freecam's registration with the mod manager and its controller.
// - state: the controller and the mod manager link, game thread only
// - handoff: snapshot, legend, actions and timings between the game thread and the manager's render-thread frame
//   handler, locked only to copy in or out; the game thread uses try_lock on it and never waits
// - clock: what the frame handler saw last: frames drawn, screen size, pointer over the UI
// Lock order: state before handoff; handoff is always the last lock taken
pub struct FreecamSession {
    state: Mutex<HandleState>,
    handoff: Arc<Mutex<UiHandoff>>,
    clock: Arc<RenderClock>,
    // set once the first poisoned lock was logged, so a panic gives one log line and not one per call
    poison_logged: AtomicBool,
}

impl FreecamSession {
    // starts the link to the mod manager with freecam's egui pass; call it in `enable`. The link connects itself when
    // the manager's tables are published, and the first frame after that reads config.hcl
    pub fn start(ctx: &Ctx) -> Result<FreecamSession, LinkError> {
        let handoff = Arc::new(Mutex::new(UiHandoff::new()));
        let clock = Arc::new(RenderClock::new());
        let handler = FreecamFrameHandler::new(Arc::clone(&handoff), Arc::clone(&clock));
        let link = ManagerLink::start(ctx, FreecamDescriptor::DESCRIPTOR, Some(handler))?;
        let state = HandleState::new(LinkSession::new(link), ctx.log(), ctx.supports(Area::OVERLAY_SCREENSHOT));
        Ok(Self { state: Mutex::new(state), handoff, clock, poison_logged: AtomicBool::new(false) })
    }

    // game thread: connects once the manager is there, drains the manager's events and the queued inputs into core,
    // answers a start with the flight leases, runs the core frame, then applies the cursor and gives the leases back
    // once freecam is off. `lingering` names the actions whose manager keybind is dropped. `markers` receives the frame's marker batch (cleared first). None when a panic poisoned
    // the state lock
    pub fn frame(
        &self,
        status: &FrameStatus,
        inputs: &[InputItem],
        lingering: u16,
        markers: &mut Vec<MarkerEntry>,
    ) -> Option<SessionFrame> {
        let mut status = *status;
        status.present_count = self.clock.frames();
        if !(status.aspect.is_finite() && status.aspect > 0.0)
            && let Some(aspect) = self.clock.aspect()
        {
            status.aspect = aspect;
        }
        let over_ui = self.clock.pointer_over_ui();
        self.with_state(|state| {
            let mut keys: Option<KeyConfig> = None;
            if let Some(config) = state.connect_if_ready() {
                self.clock.set_measure(config.telemetry);
                keys = Some(config.keys.sanitized());
            }
            state.controller_mut().set_pointer_over_ui(over_ui);
            if let Some(changed) = state.apply_events(lingering) {
                keys = Some(changed);
            }
            state.apply_inputs(inputs);
            state.start_if_asked();
            if let Some(mut handoff) = self.try_handoff() {
                handoff.apply_actions(state.controller_mut());
            }
            let mut status = status;
            state.complete_status(&mut status);
            let output: FrameOutput = state.controller_mut().frame(&status);
            if let Some(mut handoff) = self.try_handoff() {
                handoff.publish(state.controller_mut());
            }
            state.apply_output(&output);
            markers.clear();
            markers.extend_from_slice(state.publish_markers());
            let point_count = state.controller_mut().path().keys().len();
            SessionFrame { output, keys, holding: state.holding(), panel_open: state.panel_open(), point_count }
        })
    }

    // a camera instruction reached the client. None when a panic poisoned the state lock
    pub fn server_instruction(&self, kind: ServerInstruction, from_other_mod: bool) -> Option<()> {
        self.with_state(|state| state.server_instruction(kind, from_other_mod))
    }

    // the answer to the ray the last frame asked for: the hit point, or None for a miss
    pub fn ray_result(&self, hit: Option<Vec3>) -> Option<()> {
        self.with_state(|state| state.ray_result(hit))
    }

    // the number of markers a batch can hold, for the buffers of the caller
    pub fn marker_capacity(&self) -> usize {
        self.with_state(|state| state.controller_mut().marker_capacity()).unwrap_or(0)
    }

    fn with_state<R>(&self, body: impl FnOnce(&mut HandleState) -> R) -> Option<R> {
        match self.state.lock() {
            Ok(mut state) => Some(body(&mut state)),
            Err(_) => {
                self.log_poison("freecam controller");
                None
            }
        }
    }

    fn try_handoff(&self) -> Option<MutexGuard<'_, UiHandoff>> {
        match self.handoff.try_lock() {
            Ok(guard) => Some(guard),
            Err(TryLockError::WouldBlock) => None,
            Err(TryLockError::Poisoned(_)) => {
                self.log_poison("UI hand-off");
                None
            }
        }
    }

    fn log_poison(&self, name: &str) {
        if !self.poison_logged.swap(true, Ordering::Relaxed) {
            curia::error!("lock is poisoned by an earlier panic; calls that need it now fail", { lock: name });
        }
    }
}
