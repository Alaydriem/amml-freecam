use freecam_core::structs::ServerInstruction;

// what a camera instruction asks of the client: the free camera on (a set), the player camera back (a clear), or
// neither
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionKind {
    Set,
    Clear,
    Other,
}

impl InstructionKind {
    // what core is told of a server instruction
    pub fn server(self) -> ServerInstruction {
        match self {
            Self::Set | Self::Clear => ServerInstruction::SetOrClear,
            Self::Other => ServerInstruction::Other,
        }
    }
}
