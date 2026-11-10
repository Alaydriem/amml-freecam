use std::ffi::c_void;
use std::sync::{Mutex, PoisonError};

use freecam_core::structs::CameraPose;

use crate::game::{InstructionLayout, PacketHandler, PresetList};
use crate::services::PacketCapture;
use crate::types::{DeliveringScope, InstructionKind, InstructionState};

// the camera instructions the mod gives the client's own packet handler: set the free preset at a pose, or clear.
// One CameraInstructionPacket serves the whole session; each delivery writes only the set or the clear member
pub struct CameraInstructions {
    layout: InstructionLayout,
    handler: PacketHandler,
    presets: PresetList,
    state: Mutex<InstructionState>,
}

impl CameraInstructions {
    pub fn new(layout: InstructionLayout, handler: PacketHandler, presets: PresetList) -> Self {
        Self { layout, handler, presets, state: Mutex::new(InstructionState::default()) }
    }

    /// once per frame: finds the free preset in the level's list. Ready means a handler is captured and the preset
    /// exists
    ///
    /// # Safety
    /// `level` must be null or the live Level of the local player, on the game thread
    #[allow(unsafe_code)]
    pub unsafe fn ready(&self, capture: &PacketCapture, level: *mut c_void) -> bool {
        // SAFETY: forwarded to the caller
        let found = unsafe { self.presets.index_of(level, PresetList::FREE_PRESET) };
        self.lock().free_index = found;
        capture.captured() && found.is_some()
    }

    // sets the free preset at `pose`; false when the preset is not known, no handler is captured or this is not the
    // capture thread
    pub fn send_set(&self, capture: &PacketCapture, pose: &CameraPose) -> bool {
        let mut state = self.lock();
        let Some(index) = state.free_index else { return false };
        let position = [pose.position.x, pose.position.y, pose.position.z];
        self.deliver(capture, &mut state, |layout, packet| {
            // SAFETY: the packet is the live CameraInstructionPacket the factory made for these entries
            #[allow(unsafe_code)]
            unsafe {
                layout.write_set(packet, index, position, [pose.pitch, pose.yaw]);
            }
        })
    }

    // clears the camera back to the player; false when no handler is captured or this is not the capture thread
    pub fn send_clear(&self, capture: &PacketCapture) -> bool {
        let mut state = self.lock();
        self.deliver(capture, &mut state, |layout, packet| {
            // SAFETY: as in send_set
            #[allow(unsafe_code)]
            unsafe {
                layout.write_clear(packet);
            }
        })
    }

    /// what a CameraInstructionPacket the client handles asks for
    ///
    /// # Safety
    /// `packet` must be null or a live CameraInstructionPacket of the game build the entries were resolved for
    #[allow(unsafe_code)]
    pub unsafe fn kind_of(&self, packet: *const c_void) -> InstructionKind {
        if packet.is_null() {
            return InstructionKind::Other;
        }
        // SAFETY: forwarded to the caller
        unsafe { self.layout.kind(packet) }
    }

    /// a camera instruction reached the client handler: `report` gets its kind, unless this thread is inside the
    /// mod's own delivery
    ///
    /// # Safety
    /// As `kind_of`
    #[allow(unsafe_code)]
    pub unsafe fn report(&self, packet: *const c_void, report: impl FnOnce(InstructionKind)) {
        if DeliveringScope::active() {
            return;
        }
        // SAFETY: forwarded to the caller
        report(unsafe { self.kind_of(packet) });
    }

    // disable: the preset index is forgotten and the packet goes back to the game
    pub fn reset(&self) {
        let mut state = self.lock();
        state.free_index = None;
        state.packet.release();
    }

    // the packet is made on the first delivery only, once a handler is captured on this thread
    fn deliver(
        &self,
        capture: &PacketCapture,
        state: &mut InstructionState,
        write: impl FnOnce(&InstructionLayout, *mut c_void),
    ) -> bool {
        if !capture.captured() || !capture.on_capture_thread() {
            return false;
        }
        let Some(packet) = state.packet.get_or_make(|| self.layout.create_packet()) else {
            curia::warn!("could not create a camera instruction packet");
            return false;
        };
        write(&self.layout, packet);
        let _delivering = DeliveringScope::enter();
        capture.deliver(|handler, id| {
            // SAFETY: the capture holds a live handler and its id under its lock, on the capture thread; the packet
            // is live
            #[allow(unsafe_code)]
            unsafe {
                self.handler.deliver_instruction(handler, id, packet)
            }
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, InstructionState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
