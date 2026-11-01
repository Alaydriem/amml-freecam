// what a camera instruction packet from the server asks for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerInstruction {
    SetOrClear,
    Other,
}
