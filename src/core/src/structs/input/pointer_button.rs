// a mouse button as the mod layer reports it; only the right button starts a look drag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
    Other,
}
