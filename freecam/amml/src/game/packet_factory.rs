use std::ffi::c_void;

use amml_game::game::RawObject;
use amml_msvc::SharedPtr;

// the game's packet factory, `MinecraftPackets::createPacket`, with a type check: every packet it makes is checked
// by its `Packet::getName` before the mod writes to it
#[derive(Debug, Clone, Copy)]
pub struct PacketFactory {
    create: usize,
}

// `std::shared_ptr<Packet> MinecraftPackets::createPacket(MinecraftPacketIds)`, static: the buffer in rcx, the id in
// edx, the buffer back in rax
pub type CreatePacketFn = unsafe extern "C" fn(*mut SharedPtr<c_void>, u32) -> *mut SharedPtr<c_void>;

// `std::string_view Packet::getName() const`: this, then the buffer for the view (pointer, length)
type GetNameFn = unsafe extern "C" fn(*const c_void, *mut [usize; 2]) -> *mut [usize; 2];

impl PacketFactory {
    // MinecraftPacketIds: network protocol numbers, which the game appends to and does not renumber
    pub const CAMERA_INSTRUCTION_ID: u32 = 300;
    pub const PRIMITIVE_SHAPES_ID: u32 = 328;
    pub const CAMERA_INSTRUCTION_NAME: &'static str = "CameraInstructionPacket";
    pub const PRIMITIVE_SHAPES_NAME: &'static str = "PrimitiveShapesPacket";
    // Packet's virtuals: the destructor, getId, getName
    const GET_NAME_SLOT: u64 = 2;

    pub fn new(create: usize) -> Self {
        Self { create }
    }

    // a new packet of `id` with the game's control block; None when the factory gives none or the packet's name is
    // not `name` (the packet is then given back)
    #[allow(unsafe_code)]
    pub fn create(&self, id: u32, name: &str) -> Option<SharedPtr<c_void>> {
        if self.create == 0 {
            return None;
        }
        // SAFETY: the entry is the address of `MinecraftPackets::createPacket`
        let create: CreatePacketFn = unsafe { std::mem::transmute(self.create) };
        let mut out = std::mem::MaybeUninit::<SharedPtr<c_void>>::zeroed();
        // SAFETY: `out` is a buffer for a shared_ptr; the factory constructs it, empty for an unknown id
        unsafe { create(out.as_mut_ptr(), id) };
        // SAFETY: the factory wrote a shared_ptr (or left the zeroed, empty one)
        let mut packet = unsafe { out.assume_init() };
        let pointer = std::ptr::from_ref(packet.get()?);
        // SAFETY: a packet from the factory, whose vtable starts with Packet's virtuals
        if unsafe { Self::name_of(pointer) }.as_deref() == Some(name.as_bytes()) {
            return Some(packet);
        }
        curia::warn!("the game made a packet of another type; it is not used", { id: id, expected: name });
        // SAFETY: the packet came from the factory with one use, which is given back
        unsafe { packet.release() };
        None
    }

    /// the name the packet gives through `Packet::getName`; None for a null vtable or slot
    ///
    /// # Safety
    /// `packet` must be a live packet of the game
    #[allow(unsafe_code)]
    pub unsafe fn name_of(packet: *const c_void) -> Option<Vec<u8>> {
        // SAFETY: the caller guarantees a live packet, whose vtable holds getName at the slot
        let function = unsafe { RawObject::slot(packet.cast_mut(), Self::GET_NAME_SLOT) }?;
        // SAFETY: the slot holds `std::string_view getName() const`
        let get_name: GetNameFn = unsafe { std::mem::transmute(function) };
        let mut view = [0usize; 2];
        // SAFETY: `view` is the 16-byte buffer for the string_view
        unsafe { get_name(packet, &mut view) };
        let [text, length] = view;
        if text == 0 || length > 256 {
            return None;
        }
        // SAFETY: the view points to the packet's static name of `length` bytes
        Some(unsafe { std::slice::from_raw_parts(text as *const u8, length) }.to_vec())
    }
}
