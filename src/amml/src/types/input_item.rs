use amm_core::structs::event::KeybindPhase;
use freecam_core::structs::{FreecamAction, PointerButton};

// one input the listeners hand to the frame: a matched bind, raw mouse counts, wheel notches or a mouse button
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputItem {
    Bind(FreecamAction, KeybindPhase),
    Look { dx: f32, dy: f32 },
    Wheel(f32),
    Button(PointerButton, bool),
}
