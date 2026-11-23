#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreecamState {
    Off,
    On,
    Playing,
    Orbiting,
    Exiting { frames_left: u32 },
}
