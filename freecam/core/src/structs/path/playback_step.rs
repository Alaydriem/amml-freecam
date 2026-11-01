#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackStep {
    pub distance: f32,
    pub settling: bool,
    pub finished: bool,
}
