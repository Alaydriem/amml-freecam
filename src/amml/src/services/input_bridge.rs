use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use amml_api::amml_abi::{KeyInput, MouseInput};
use amml_api::{Cancel, Event, KeyInputExt, Logger, Modifiers, MouseAction, MouseButton, MouseInputExt};
use freecam_core::config::KeyConfig;
use freecam_core::structs::{FreecamAction, PointerButton};

use crate::services::{BindMatcher, InputQueue};
use crate::types::InputItem;

// the path of the key and mouse listeners to the frame: real keys go through the bind matcher, real mouse moves,
// wheel notches and buttons go to the queue while freecam holds its Input lease. The listeners may run on the input
// thread; the frame drains the queue and sets the matcher's gate. Lock order: the frame driver, then the matcher
pub struct InputBridge {
    matcher: Mutex<BindMatcher>,
    queue: InputQueue,
    active: AtomicBool,
    // the lease is held and the panel closed: the mouse drives the camera
    mouse_open: AtomicBool,
    // the real keys logged since the gate last opened, so a run shows whether keys reach freecam under its lease
    keys_logged: AtomicU32,
    // the last gate inputs (holding, panel open), logged when they change
    gate: Mutex<Option<(bool, bool)>>,
}

impl InputBridge {
    pub fn new() -> Self {
        Self {
            matcher: Mutex::new(BindMatcher::new(&KeyConfig::default())),
            queue: InputQueue::new(InputQueue::CAPACITY),
            active: AtomicBool::new(false),
            mouse_open: AtomicBool::new(false),
            keys_logged: AtomicU32::new(0),
            gate: Mutex::new(None),
        }
    }

    // the real keys logged per opening of the gate
    pub const KEYS_LOGGED: u32 = 4;

    // a `KeyInput` listener call; `modifiers` from `Input::modifiers` (the event carries no F3). Injected keys are
    // not the user's and match nothing
    pub fn on_key(&self, event: &KeyInput, modifiers: Modifiers, log: &Logger) {
        if event.is_injected() {
            return;
        }
        if self.active.load(Ordering::Relaxed) && self.keys_logged.fetch_add(1, Ordering::Relaxed) < Self::KEYS_LOGGED {
            log.debug(
                "flight key seen",
                &[
                    ("key", &format!("{:#04x}", event.key)),
                    ("down", &event.down.to_string()),
                    ("repeat", &event.repeat.to_string()),
                    ("cancelled", &u8::from(event.is_cancelled()).to_string()),
                ],
            );
        }
        self.matcher().on_key(event.key_code(), event.down != 0, event.repeat != 0, modifiers, &self.queue);
    }

    // a `MouseInput` listener call: raw counts, notches and buttons, only while freecam holds its Input lease and the
    // manager panel is closed (the panel takes the mouse then). The lease blocks no wheel, so the notch is cancelled
    // here: the game does not scroll the hotbar under freecam
    pub fn on_mouse(&self, event: &mut MouseInput) {
        if event.is_injected() || !self.mouse_open.load(Ordering::Relaxed) {
            return;
        }
        let item = match event.action() {
            MouseAction::Move => InputItem::Look { dx: event.dx as f32, dy: event.dy as f32 },
            MouseAction::Wheel { notches } => {
                event.cancel();
                InputItem::Wheel(notches)
            }
            MouseAction::Button { button, down } => InputItem::Button(Self::button(button), down),
            _ => return,
        };
        self.queue.push(item);
    }

    // once per frame: the binds count while freecam holds its Input lease and the panel is closed; a closed gate
    // releases the held binds. The window focus is no condition: real keys reach the game only while its window has
    // the keyboard focus
    pub fn set_gate(&self, holding: bool, panel_open: bool, log: &Logger) {
        let open = holding && !panel_open;
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if *gate != Some((holding, panel_open)) {
            *gate = Some((holding, panel_open));
            if open {
                self.keys_logged.store(0, Ordering::Relaxed);
            }
            log.debug(
                "bind gate",
                &[
                    ("open", &open.to_string()),
                    ("holding", &holding.to_string()),
                    ("panel_open", &panel_open.to_string()),
                ],
            );
        }
        drop(gate);
        self.active.store(holding, Ordering::Relaxed);
        self.mouse_open.store(open, Ordering::Relaxed);
        self.matcher().set_gate(holding, panel_open, &self.queue);
    }

    // a focus loss: every held bind goes up, so a key the user let go of in another window is not stuck down
    pub fn release_held(&self) {
        self.matcher().release_all(&self.queue);
    }

    // the actions whose manager keybind is dropped now (bit = the action's index): a bind freecam let go of while its
    // key stays held
    pub fn lingering(&self) -> u16 {
        let matcher = self.matcher();
        (0..BindMatcher::ACTIONS)
            .filter_map(|index| FreecamAction::from_index(u16::try_from(index).ok()?))
            .filter(|action| matcher.lingering(*action))
            .fold(0u16, |mask, action| mask | (1 << action.index()))
    }

    // the chords of a new config.hcl
    pub fn set_keys(&self, keys: &KeyConfig) {
        self.matcher().set_keys(keys, &self.queue);
    }

    // the inputs queued since the last frame, into `out` (cleared first)
    pub fn drain(&self, out: &mut Vec<InputItem>) {
        self.queue.drain(out);
    }

    fn button(button: MouseButton) -> PointerButton {
        match button {
            MouseButton::Left => PointerButton::Left,
            MouseButton::Right => PointerButton::Right,
            MouseButton::Middle => PointerButton::Middle,
            _ => PointerButton::Other,
        }
    }

    fn matcher(&self) -> MutexGuard<'_, BindMatcher> {
        self.matcher.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for InputBridge {
    fn default() -> Self {
        Self::new()
    }
}
