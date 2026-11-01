use crate::types::PacketSlot;

// the index of the free preset in the level's list, and the reused camera instruction packet
#[derive(Default)]
pub struct InstructionState {
    pub free_index: Option<u32>,
    pub packet: PacketSlot,
}
