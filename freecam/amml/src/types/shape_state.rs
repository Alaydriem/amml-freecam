use crate::types::PacketSlot;

// the reused PrimitiveShapesPacket, and the game-heap block that backs its shape vector with the number of payloads
// it holds
pub struct ShapeState {
    pub packet: PacketSlot,
    pub first: *mut u8,
    pub capacity: usize,
}

// SAFETY: the packet and its block are used only on the thread that delivers, behind the lock of the shape delivery
#[allow(unsafe_code)]
unsafe impl Send for ShapeState {}

impl ShapeState {
    pub const fn new() -> Self {
        Self { packet: PacketSlot::new(), first: std::ptr::null_mut(), capacity: 0 }
    }
}

impl Default for ShapeState {
    fn default() -> Self {
        Self::new()
    }
}
