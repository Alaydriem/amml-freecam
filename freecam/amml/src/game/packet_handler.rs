use std::ffi::c_void;

use amml_game::GameEntries;

use crate::types::FreecamEntry;

// the client's ClientNetworkHandler: where its handle overloads sit in its vtable, and the calls of them
#[derive(Debug, Clone, Copy)]
pub struct PacketHandler {
    vtable: usize,
    chunk_publisher: usize,
    camera_presets: usize,
    camera_instruction: usize,
    primitive_shapes: Option<usize>,
}

// `void handle(NetworkIdentifier const&, Packet const&)`: this, the id, the packet
pub type HandleFn = unsafe extern "C" fn(*mut c_void, *const c_void, *const c_void);

impl PacketHandler {
    // None when a delivery entry is missing; the shapes slot is optional (markers)
    pub fn new(entries: &GameEntries) -> Option<Self> {
        let get = |id| usize::try_from(entries.get(id)?).ok();
        Some(Self {
            vtable: get(FreecamEntry::CLIENT_NETWORK_HANDLER_VTABLE)?,
            chunk_publisher: get(FreecamEntry::HANDLE_CHUNK_PUBLISHER)?,
            camera_presets: get(FreecamEntry::HANDLE_CAMERA_PRESETS)?,
            camera_instruction: get(FreecamEntry::HANDLE_CAMERA_INSTRUCTION)?,
            primitive_shapes: get(FreecamEntry::HANDLE_PRIMITIVE_SHAPES),
        })
    }

    // the addresses to hook: the ChunkPublisher, CameraPresets and CameraInstruction overloads in the class vtable.
    // None for a null vtable or slot. Called once, when the hooks are installed
    #[allow(unsafe_code)]
    pub fn hook_targets(&self) -> Option<[u64; 3]> {
        if self.vtable == 0 {
            return None;
        }
        let at = |slot: usize| {
            // SAFETY: the vtable entry is the address of the class's primary vtable in the game image, which holds
            // every handle overload the entries name
            let function = unsafe { (self.vtable as *const *const c_void).wrapping_add(slot).read() };
            (!function.is_null()).then_some(function as u64)
        };
        Some([at(self.chunk_publisher)?, at(self.camera_presets)?, at(self.camera_instruction)?])
    }

    /// calls `handle(id, packet)` for a CameraInstructionPacket on `handler` through its own vtable
    ///
    /// # Safety
    /// `handler` must be a live ClientNetworkHandler, `id` the NetworkIdentifier it was called with and `packet` a
    /// live CameraInstructionPacket, on the thread that handles the server's packets
    #[allow(unsafe_code)]
    pub unsafe fn deliver_instruction(&self, handler: *mut c_void, id: *const c_void, packet: *const c_void) -> bool {
        // SAFETY: forwarded to the caller
        unsafe { Self::call(handler, self.camera_instruction, id, packet) }
    }

    /// calls `handle(id, packet)` for a PrimitiveShapesPacket; false when the markers slot is off
    ///
    /// # Safety
    /// As `deliver_instruction`, for a live PrimitiveShapesPacket
    #[allow(unsafe_code)]
    pub unsafe fn deliver_shapes(&self, handler: *mut c_void, id: *const c_void, packet: *const c_void) -> bool {
        let Some(slot) = self.primitive_shapes else { return false };
        // SAFETY: forwarded to the caller
        unsafe { Self::call(handler, slot, id, packet) }
    }

    #[allow(unsafe_code)]
    unsafe fn call(handler: *mut c_void, slot: usize, id: *const c_void, packet: *const c_void) -> bool {
        if handler.is_null() || packet.is_null() {
            return false;
        }
        // SAFETY: the caller guarantees a live handler, whose first qword is its vtable
        let vtable = unsafe { handler.cast::<*const *const c_void>().read() };
        if vtable.is_null() {
            return false;
        }
        // SAFETY: the vtable holds the handle overload at `slot`
        let function = unsafe { vtable.add(slot).read() };
        if function.is_null() {
            return false;
        }
        // SAFETY: the slot holds a handle overload with this signature
        let handle: HandleFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the caller guarantees the arguments
        unsafe { handle(handler, id, packet) };
        true
    }
}
