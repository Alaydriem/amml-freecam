use std::ffi::c_void;

use amml_game::GameEntries;
use amml_msvc::{Optional, SharedPtr};

use crate::game::PacketFactory;
use crate::game::layout::{CameraInstructionLayout, SetInstructionLayout};
use crate::types::{FreecamEntry, InstructionKind};

// the CameraInstructionPacket: how to make one and where its instruction sits. The instruction is written through
// the CameraInstructionLayout mirror, whose front (`CameraInstruction.mSpline`) and whole
// (`CameraInstructionPacket.size`) are checked in `new`
#[derive(Debug, Clone, Copy)]
pub struct InstructionLayout {
    factory: PacketFactory,
    instruction: usize,
}

impl InstructionLayout {
    pub fn new(entries: &GameEntries) -> Option<Self> {
        let get = |id| usize::try_from(entries.get(id)?).ok();
        let instruction = get(FreecamEntry::INSTRUCTION_PACKET_INSTRUCTION)?;
        let whole = (instruction as u64)
            .checked_add(size_of::<CameraInstructionLayout>() as u64)?
            .checked_add(CameraInstructionLayout::PACKET_TAIL)?;
        let fits = entries.get(FreecamEntry::INSTRUCTION_SPLINE)? == CameraInstructionLayout::SPLINE
            && entries.get(FreecamEntry::INSTRUCTION_PACKET_SIZE)? == whole;
        let factory = PacketFactory::new(get(FreecamEntry::CREATE_PACKET)?);
        fits.then_some(Self { factory, instruction })
    }

    // a new CameraInstructionPacket from the game's factory, with the game's control block; None when the game
    // returns none or another packet
    pub fn create_packet(&self) -> Option<SharedPtr<c_void>> {
        self.factory.create(PacketFactory::CAMERA_INSTRUCTION_ID, PacketFactory::CAMERA_INSTRUCTION_NAME)
    }

    /// the set instruction for the free preset at `index`: position and rotation (pitch, yaw), every other set option
    /// disengaged and `clear` disengaged. No other member of the instruction is written
    ///
    /// # Safety
    /// `packet` must be a live CameraInstructionPacket of the game build the entries were resolved for
    #[allow(unsafe_code)]
    pub unsafe fn write_set(&self, packet: *mut c_void, index: u32, position: [f32; 3], rotation: [f32; 2]) {
        let instruction = self.instruction(packet);
        // SAFETY: the caller guarantees a live packet whose instruction is laid out as the mirror (checked in `new`);
        // both members are trivially copyable
        unsafe {
            (*instruction).set = Optional::some(SetInstructionLayout::new(index, position, rotation));
            (*instruction).clear = Optional::none();
        }
    }

    /// the clear instruction: `clear` engaged and true, the set instruction disengaged
    ///
    /// # Safety
    /// As `write_set`
    #[allow(unsafe_code)]
    pub unsafe fn write_clear(&self, packet: *mut c_void) {
        let instruction = self.instruction(packet);
        // SAFETY: as in `write_set`
        unsafe {
            (*instruction).set = Optional::none();
            (*instruction).clear = Optional::some(true);
        }
    }

    /// what `packet` asks for: a clear, a set, or neither; read by the instruction detour
    ///
    /// # Safety
    /// As `write_set`
    #[allow(unsafe_code)]
    pub unsafe fn kind(&self, packet: *const c_void) -> InstructionKind {
        let instruction = self.instruction(packet.cast_mut());
        // SAFETY: as in `write_set`
        let (set, clear) = unsafe { ((*instruction).set.get().is_some(), (*instruction).clear.get().copied()) };
        match (set, clear) {
            (_, Some(true)) => InstructionKind::Clear,
            (true, _) => InstructionKind::Set,
            _ => InstructionKind::Other,
        }
    }

    fn instruction(&self, packet: *mut c_void) -> *mut CameraInstructionLayout {
        packet.cast::<u8>().wrapping_add(self.instruction).cast()
    }
}
