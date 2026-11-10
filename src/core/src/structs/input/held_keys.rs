#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeldKeys {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub fast: bool,
}
