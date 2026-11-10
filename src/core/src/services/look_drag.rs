// the mod manager's number for the right pointer button
const BUTTON_RIGHT: u8 = 1;

// the right-button look drag in bar mode: the button goes down away from the UI, the cursor goes away and the mouse
// turns the camera like mouse-look until the button comes up, then the cursor comes back. The drag lives only inside
// bar mode
pub struct LookDrag {
    // the right button is down and went down where a drag may start
    held: bool,
    // the cursor is off for the drag
    active: bool,
}

impl LookDrag {
    pub fn new() -> Self {
        Self { held: false, active: false }
    }

    // a pointer button event; a right press starts a drag only when `can_start`, a right release always ends one
    pub fn on_button(&mut self, button: u8, down: bool, can_start: bool) {
        if button != BUTTON_RIGHT {
            return;
        }
        if !down {
            self.held = false;
        } else if can_start {
            self.held = true;
        }
    }

    // true while the cursor is off for the drag and the mouse turns the camera
    pub fn is_active(&self) -> bool {
        self.active
    }

    // once per frame, after bar mode is known for the frame; leaving bar mode drops the drag
    pub fn update(&mut self, cursor_mode: bool) {
        if !cursor_mode {
            self.held = false;
        }
        self.active = self.held;
    }
}

impl Default for LookDrag {
    fn default() -> Self {
        Self::new()
    }
}
