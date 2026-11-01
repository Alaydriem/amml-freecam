use std::cell::Cell;
use std::ffi::c_void;

use parking_lot::ReentrantMutex;

use crate::services::ThreadGate;
use crate::types::{CaptureState, ThreadId};

// the client's ClientNetworkHandler, captured from the packets the server always sends, and the deliveries to it.
// One re-entrant lock covers the state and every delivery: a delivery can re-enter a capture detour on the same
// thread, and a drop on another thread waits until the handler call returns
pub struct PacketCapture {
    state: ReentrantMutex<Cell<CaptureState>>,
    gate: ThreadGate,
}

impl PacketCapture {
    pub const fn new() -> Self {
        Self {
            state: ReentrantMutex::new(Cell::new(CaptureState {
                handler: 0,
                id: 0,
                had_player: false,
                captures: 0,
                thread: 0,
            })),
            gate: ThreadGate::new("delivery"),
        }
    }

    // a handle detour saw `handler` with `id`, on the current thread. A new handler raises the generation and is
    // logged with its thread
    pub fn capture(&self, handler: *mut c_void, id: *const c_void) {
        let guard = self.state.lock();
        let mut state = guard.get();
        if state.handler != handler as usize {
            state.captures = state.captures.wrapping_add(1);
            state.thread = ThreadId::current();
            curia::info!("captured client packet handler", { generation: state.captures, thread: state.thread });
        }
        state.handler = handler as usize;
        state.id = id as usize;
        guard.set(state);
    }

    // once per frame: when the local player went away, the handler goes with it, so it is dropped
    pub fn observe_local_player(&self, present: bool) {
        let guard = self.state.lock();
        let mut state = guard.get();
        if state.had_player && !present {
            state.handler = 0;
            state.id = 0;
        }
        state.had_player = present;
        guard.set(state);
    }

    pub fn captured(&self) -> bool {
        self.state.lock().get().is_captured()
    }

    // the capture count while a handler is captured; 0 when none is
    pub fn generation(&self) -> u32 {
        let state = self.state.lock().get();
        if state.is_captured() { state.captures } else { 0 }
    }

    // the handler generation core sees: the capture count, but 0 when no handler is captured or this is not the
    // capture thread, so core sends nothing that cannot go out
    pub fn delivery_generation(&self) -> u32 {
        let generation = self.generation();
        if generation == 0 || !self.on_capture_thread() {
            return 0;
        }
        generation
    }

    // the thread that handled the last captured packet
    pub fn thread(&self) -> u32 {
        self.state.lock().get().thread
    }

    // whether the current thread is the capture thread; the first refusal is logged
    pub fn on_capture_thread(&self) -> bool {
        self.gate.allows(self.thread())
    }

    // the refusal lines the delivery gate logged
    pub fn refusals(&self) -> u32 {
        self.gate.warnings()
    }

    // runs `call(handler, id)` under the lock: false, with no call, when no handler is captured or the current
    // thread is not the capture thread
    pub fn deliver(&self, call: impl FnOnce(*mut c_void, *const c_void) -> bool) -> bool {
        let guard = self.state.lock();
        let state = guard.get();
        if !state.is_captured() || !self.gate.allows(state.thread) {
            return false;
        }
        call(state.handler as *mut c_void, state.id as *const c_void)
    }

    // disable: the handler is dropped and the next enable starts with no player seen
    pub fn reset(&self) {
        let guard = self.state.lock();
        let mut state = guard.get();
        state.handler = 0;
        state.id = 0;
        state.had_player = false;
        guard.set(state);
        self.gate.reset();
    }
}

impl Default for PacketCapture {
    fn default() -> Self {
        Self::new()
    }
}
