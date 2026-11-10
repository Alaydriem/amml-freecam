use std::ffi::c_void;

use amml_msvc::SharedPtr;

// one packet the game made for the mod, kept for the whole session and given back with the game's control block
pub struct PacketSlot {
    packet: Option<SharedPtr<c_void>>,
}

// SAFETY: the packet is a game object used only on the thread that delivers to the packet handler, behind the lock of
// the service that owns the slot
#[allow(unsafe_code)]
unsafe impl Send for PacketSlot {}

impl PacketSlot {
    pub const fn new() -> Self {
        Self { packet: None }
    }

    // the packet; `make` runs when none is held. None when `make` gives none
    pub fn get_or_make(&mut self, make: impl FnOnce() -> Option<SharedPtr<c_void>>) -> Option<*mut c_void> {
        if self.packet.is_none() {
            self.packet = make();
        }
        self.pointer()
    }

    pub fn pointer(&self) -> Option<*mut c_void> {
        let packet = self.packet.as_ref()?.get()?;
        Some(std::ptr::from_ref(packet).cast_mut())
    }

    // gives the use back to the game; the game destroys the packet when it held the last use
    #[allow(unsafe_code)]
    pub fn release(&mut self) {
        if let Some(mut packet) = self.packet.take() {
            // SAFETY: the packet came from the game's factory with one use, which this slot owned
            unsafe { packet.release() };
        }
    }
}

impl Default for PacketSlot {
    fn default() -> Self {
        Self::new()
    }
}
