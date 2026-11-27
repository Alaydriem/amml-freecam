use amm_core::structs::event::KeybindPhase;
use amml_api::{Chord, KeyCode, Modifiers};
use freecam_core::config::KeyConfig;
use freecam_core::structs::FreecamAction;

use crate::services::InputQueue;
use crate::types::InputItem;

// freecam's own bind matching while it holds its Input lease: the lease blocks the keys, so the manager sends no
// keybind then. Each real key is matched against the chords of config.hcl; a match goes to the input queue as the
// manager's keybind of the same action would. A flight key with no modifiers in its chord matches on its key alone,
// so W still flies with Ctrl or Shift held; every other bind needs its modifiers exactly. A flight key's repeat with no
// down seen counts as a press (a fly key held through the toggle); a one-shot bind's gives nothing, so a toggle chord
// held past the repeat delay does not toggle again. A bind still held when the matcher lets it go (the gate closed)
// lingers until its key goes up or is pressed anew, and the manager's keybind of that action is dropped meanwhile:
// the manager never saw the blocked down, so it counts the repeats as new presses
pub struct BindMatcher {
    chords: [Option<Chord>; Self::ACTIONS],
    down: [bool; Self::ACTIONS],
    lingering: [bool; Self::ACTIONS],
    active: bool,
    panel_open: bool,
}

impl BindMatcher {
    pub const ACTIONS: usize = FreecamAction::COUNT;

    pub fn new(keys: &KeyConfig) -> Self {
        Self {
            chords: Self::parse(keys),
            down: [false; Self::ACTIONS],
            lingering: [false; Self::ACTIONS],
            active: false,
            panel_open: false,
        }
    }

    // a settings change: the held binds go up, then the new chords count
    pub fn set_keys(&mut self, keys: &KeyConfig, queue: &InputQueue) {
        self.release_all(queue);
        self.chords = Self::parse(keys);
    }

    // once per frame: `active` while freecam holds its Input lease, `panel_open` while the manager panel is open. A
    // closed gate releases the held binds and holds back new presses
    pub fn set_gate(&mut self, active: bool, panel_open: bool, queue: &InputQueue) {
        self.active = active;
        self.panel_open = panel_open;
        if !self.open() {
            self.release_all(queue);
        }
    }

    // true while freecam holds its Input lease
    pub fn active(&self) -> bool {
        self.active
    }

    // one real key event; `modifiers` must carry F3 (from `Input::modifiers`)
    pub fn on_key(&mut self, key: KeyCode, down: bool, repeat: bool, modifiers: Modifiers, queue: &InputQueue) {
        if !down {
            self.release_key(key, queue);
            self.end_lingering(key);
            return;
        }
        if !repeat {
            self.end_lingering(key);
        }
        if !self.open() {
            return;
        }
        for index in 0..Self::ACTIONS {
            let (Some(action), Some(chord)) = (Self::action(index), self.chords[index]) else { continue };
            if !Self::chord_matches(action, &chord, key, modifiers) {
                continue;
            }
            let phase = match (self.down[index], repeat) {
                (false, false) => KeybindPhase::Pressed,
                (false, true) if Self::is_flight(action) => KeybindPhase::Pressed,
                (false, true) => continue,
                (true, true) => KeybindPhase::Repeat,
                (true, false) => continue,
            };
            self.down[index] = true;
            queue.push(InputItem::Bind(action, phase));
        }
    }

    // the manager's keybind of `action` is dropped while this is true
    pub fn lingering(&self, action: FreecamAction) -> bool {
        self.lingering.get(usize::from(action.index())).copied().unwrap_or(false)
    }

    fn end_lingering(&mut self, key: KeyCode) {
        for index in 0..Self::ACTIONS {
            if self.chords[index].is_some_and(|chord| Self::same_key(&chord, key)) {
                self.lingering[index] = false;
            }
        }
    }

    fn open(&self) -> bool {
        self.active && !self.panel_open
    }

    fn release_key(&mut self, key: KeyCode, queue: &InputQueue) {
        for index in 0..Self::ACTIONS {
            let same_key = self.chords[index].is_some_and(|chord| Self::same_key(&chord, key));
            if self.down[index] && same_key {
                self.release(index, queue);
            }
        }
    }

    // every held bind goes up
    pub fn release_all(&mut self, queue: &InputQueue) {
        for index in 0..Self::ACTIONS {
            if self.down[index] {
                self.lingering[index] = true;
                self.release(index, queue);
            }
        }
    }

    fn release(&mut self, index: usize, queue: &InputQueue) {
        self.down[index] = false;
        if let Some(action) = Self::action(index) {
            queue.push(InputItem::Bind(action, KeybindPhase::Released));
        }
    }

    fn chord_matches(action: FreecamAction, chord: &Chord, key: KeyCode, modifiers: Modifiers) -> bool {
        if Self::is_flight(action) && chord.modifiers == Modifiers::NONE {
            return Self::same_key(chord, key);
        }
        chord.matches(key, modifiers)
    }

    fn same_key(chord: &Chord, key: KeyCode) -> bool {
        chord.key == key || chord.key == key.generic()
    }

    fn is_flight(action: FreecamAction) -> bool {
        matches!(
            action,
            FreecamAction::FlyForward
                | FreecamAction::FlyBack
                | FreecamAction::FlyLeft
                | FreecamAction::FlyRight
                | FreecamAction::FlyUp
                | FreecamAction::FlyDown
                | FreecamAction::FlyFast
        )
    }

    fn action(index: usize) -> Option<FreecamAction> {
        FreecamAction::from_index(u16::try_from(index).ok()?)
    }

    fn parse(keys: &KeyConfig) -> [Option<Chord>; Self::ACTIONS] {
        std::array::from_fn(|index| {
            let action = Self::action(index)?;
            let text = Self::text(keys, action);
            match Chord::parse(text) {
                Ok(chord) => Some(chord),
                Err(error) => {
                    curia::warn!("a bind chord does not parse; the bind is off while freecam flies", {
                        bind: action.name(),
                        chord: text,
                        error: error.to_string(),
                    });
                    None
                }
            }
        })
    }

    fn text(keys: &KeyConfig, action: FreecamAction) -> &str {
        match action {
            FreecamAction::Toggle => &keys.toggle,
            FreecamAction::ToggleSmooth => &keys.toggle_smooth,
            FreecamAction::AddPoint => &keys.add_point,
            FreecamAction::ToggleBar => &keys.toggle_bar,
            FreecamAction::DeletePoint => &keys.delete_point,
            FreecamAction::FlyForward => &keys.fly_forward,
            FreecamAction::FlyBack => &keys.fly_back,
            FreecamAction::FlyLeft => &keys.fly_left,
            FreecamAction::FlyRight => &keys.fly_right,
            FreecamAction::FlyUp => &keys.fly_up,
            FreecamAction::FlyDown => &keys.fly_down,
            FreecamAction::FlyFast => &keys.fly_fast,
            FreecamAction::Orbit => &keys.orbit,
        }
    }
}
