use crate::services::{FrameStatusReader, FreecamGame, FreecamSession, PacketCapture, RenderOverride};

// what one frame of the driver works with: the game access, the registration with core, the capture, the render
// override, the status reader, and whether the free camera was on at the end of the last frame
#[derive(Clone, Copy)]
pub struct FrameContext<'a> {
    pub game: &'a FreecamGame,
    pub session: &'a FreecamSession,
    pub capture: &'a PacketCapture,
    pub render: &'a RenderOverride,
    pub reader: &'a FrameStatusReader,
    pub camera_owned: bool,
}
