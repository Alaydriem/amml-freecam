// the client packet handler the mod captured, and how it got there
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaptureState {
    // the ClientNetworkHandler; 0 when none is captured
    pub handler: usize,
    // the NetworkIdentifier the handler was last called with; it belongs to the connection, which lives as long as
    // the handler
    pub id: usize,
    // whether the last frame had a local player
    pub had_player: bool,
    // the number of times a new handler was captured: the handler generation
    pub captures: u32,
    // the thread that handled the last captured packet
    pub thread: u32,
}

impl CaptureState {
    pub fn is_captured(&self) -> bool {
        self.handler != 0 && self.id != 0
    }
}
