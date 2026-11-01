use serde::{Deserialize, Serialize};

const TAB: &str = "Tab";
const DEFAULT_TOGGLE_BAR: &str = "F3+E";

// the keybind chords as config.hcl stores them; the mod manager matches them and sends the actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    pub toggle: String,
    pub toggle_smooth: String,
    pub add_point: String,
    pub toggle_bar: String,
    pub delete_point: String,
    pub fly_forward: String,
    pub fly_back: String,
    pub fly_left: String,
    pub fly_right: String,
    pub fly_up: String,
    pub fly_down: String,
    pub fly_fast: String,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            toggle: "F3+P".to_owned(),
            toggle_smooth: "Ctrl+LeftBracket".to_owned(),
            add_point: "Ctrl+B".to_owned(),
            toggle_bar: DEFAULT_TOGGLE_BAR.to_owned(),
            delete_point: "F3+Delete".to_owned(),
            fly_forward: "W".to_owned(),
            fly_back: "S".to_owned(),
            fly_left: "A".to_owned(),
            fly_right: "D".to_owned(),
            fly_up: "Space".to_owned(),
            fly_down: "Shift".to_owned(),
            fly_fast: "Ctrl".to_owned(),
        }
    }
}

impl KeyConfig {
    // returns a copy with every chord moved to a usable one
    pub fn sanitized(self) -> Self {
        if !self.toggle_bar.eq_ignore_ascii_case(TAB) {
            return self;
        }
        curia::warn!("keys.toggle_bar is Tab, which the game uses; using the default chord", {
            chord: DEFAULT_TOGGLE_BAR
        });
        Self { toggle_bar: DEFAULT_TOGGLE_BAR.to_owned(), ..self }
    }
}
