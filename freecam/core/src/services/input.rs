use crate::structs::{FreecamAction, HeldKeys};

// the flight keys the mod manager holds down, and the look and wheel input since the last frame
pub struct InputTracker {
    held: HeldKeys,
    look_x: f32,
    look_y: f32,
    scroll: i32,
}

impl InputTracker {
    pub fn new() -> Self {
        Self { held: HeldKeys::default(), look_x: 0.0, look_y: 0.0, scroll: 0 }
    }

    // a flight keybind went down or up; any other action changes nothing
    pub fn on_flight_key(&mut self, action: FreecamAction, down: bool) {
        let held = &mut self.held;
        let slot = match action {
            FreecamAction::FlyForward => &mut held.forward,
            FreecamAction::FlyBack => &mut held.back,
            FreecamAction::FlyLeft => &mut held.left,
            FreecamAction::FlyRight => &mut held.right,
            FreecamAction::FlyUp => &mut held.up,
            FreecamAction::FlyDown => &mut held.down,
            FreecamAction::FlyFast => &mut held.fast,
            _ => return,
        };
        *slot = down;
    }

    pub fn on_pointer_delta(&mut self, dx: f32, dy: f32, active: bool, menu_open: bool) {
        if active && !menu_open {
            self.look_x += dx;
            self.look_y += dy;
        }
    }

    // one speed step per wheel event, by its sign
    pub fn on_wheel(&mut self, delta: f32, active: bool, menu_open: bool) {
        if active && !menu_open && delta != 0.0 {
            self.scroll += if delta > 0.0 { 1 } else { -1 };
        }
    }

    pub fn take_look(&mut self) -> (f32, f32) {
        let look = (self.look_x, self.look_y);
        self.look_x = 0.0;
        self.look_y = 0.0;
        look
    }

    pub fn take_scroll(&mut self) -> i32 {
        std::mem::take(&mut self.scroll)
    }

    pub fn held(&self) -> HeldKeys {
        self.held
    }

    pub fn clear_held(&mut self) {
        self.held = HeldKeys::default();
    }
}

impl Default for InputTracker {
    fn default() -> Self {
        Self::new()
    }
}
