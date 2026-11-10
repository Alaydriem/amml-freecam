// what the fake instruction handler saw for one packet
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSeen {
    // whether the mod marked the call as its own delivery
    pub delivering: bool,
    pub set: bool,
    pub clear: bool,
    pub bytes: Vec<u8>,
}
