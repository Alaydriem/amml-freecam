use std::ffi::c_void;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::game::CameraRegistry;
use crate::types::{InstructionKind, WatchState};

// whether a player camera is the one the game shows. Two sources: the camera registry (the active camera entity, as
// the LeviLamina build read it; the camera watch feature) and the server's camera instructions (a set means a server
// camera is on, a clear that the player camera is back). The registry wins when it can be read; without it the
// instructions decide. A disagreement that lasts more than a second is logged once, except while freecam holds the
// camera: the active camera is then the free one by the mod's own instruction, which the flag does not see
pub struct CameraWatch {
    registry: Option<CameraRegistry>,
    state: Mutex<WatchState>,
}

impl CameraWatch {
    // how long the two sources may disagree before one line is logged
    pub const DISAGREE_SECONDS: f64 = 1.0;

    pub fn new(registry: Option<CameraRegistry>) -> Self {
        Self { registry, state: Mutex::new(WatchState::default()) }
    }

    pub fn is_on(&self) -> bool {
        self.registry.is_some()
    }

    // a camera instruction from the server reached the client handler (the mod's own never come here)
    pub fn on_server_instruction(&self, kind: InstructionKind) {
        let mut state = self.lock();
        match kind {
            InstructionKind::Set => state.server_camera = true,
            InstructionKind::Clear => state.server_camera = false,
            InstructionKind::Other => {}
        }
    }

    /// whether a player camera is active this frame. `generation` is the capture generation: a new handler (a new
    /// world) or none (no world) starts with the player camera. Game thread
    ///
    /// # Safety
    /// `client` must be null or the live ClientInstance
    #[allow(unsafe_code)]
    pub unsafe fn player_camera_active(
        &self,
        client: *mut c_void,
        generation: u32,
        now: f64,
        own_camera: bool,
    ) -> bool {
        // SAFETY: forwarded to the caller
        let registry = self.registry.as_ref().and_then(|registry| unsafe { registry.player_camera_active(client) });
        self.decide(registry, generation, now, own_camera)
    }

    // the decision of one frame from the registry read (None: no read); `own_camera` while freecam holds the camera
    pub fn decide(&self, registry: Option<bool>, generation: u32, now: f64, own_camera: bool) -> bool {
        let mut state = self.lock();
        if state.generation != generation {
            *state = WatchState { generation, ..WatchState::default() };
        }
        let from_instructions = !state.server_camera;
        let Some(from_registry) = registry else {
            state.disagree_since = None;
            return from_instructions;
        };
        if from_registry == from_instructions || own_camera {
            state.disagree_since = None;
            state.disagree_logged = false;
            return from_registry;
        }
        let since = *state.disagree_since.get_or_insert(now);
        if now - since > Self::DISAGREE_SECONDS && !state.disagree_logged {
            state.disagree_logged = true;
            curia::warn!("the camera registry and the server camera instructions disagree", {
                registry_player_camera: from_registry,
                instructions_player_camera: from_instructions,
            });
        }
        from_registry
    }

    // disable: the flag goes
    pub fn reset(&self) {
        *self.lock() = WatchState::default();
    }

    fn lock(&self) -> MutexGuard<'_, WatchState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
