// a keybind freecam declares to the mod manager; the order is the descriptor's keybind order, which is how the
// manager names an action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreecamAction {
    Toggle,
    ToggleSmooth,
    AddPoint,
    ToggleBar,
    DeletePoint,
    FlyForward,
    FlyBack,
    FlyLeft,
    FlyRight,
    FlyUp,
    FlyDown,
    FlyFast,
}

impl FreecamAction {
    const ALL: [FreecamAction; 12] = [
        Self::Toggle,
        Self::ToggleSmooth,
        Self::AddPoint,
        Self::ToggleBar,
        Self::DeletePoint,
        Self::FlyForward,
        Self::FlyBack,
        Self::FlyLeft,
        Self::FlyRight,
        Self::FlyUp,
        Self::FlyDown,
        Self::FlyFast,
    ];

    // the action of the manager's keybind index, None past the last one
    pub fn from_index(index: u16) -> Option<FreecamAction> {
        Self::ALL.get(usize::from(index)).copied()
    }

    // the manager's keybind index of the action
    pub fn index(self) -> u16 {
        self as u16
    }

    // the keybind's name in the descriptor
    pub fn name(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::ToggleSmooth => "toggle_smooth",
            Self::AddPoint => "add_point",
            Self::ToggleBar => "toggle_bar",
            Self::DeletePoint => "delete_point",
            Self::FlyForward => "fly_forward",
            Self::FlyBack => "fly_back",
            Self::FlyLeft => "fly_left",
            Self::FlyRight => "fly_right",
            Self::FlyUp => "fly_up",
            Self::FlyDown => "fly_down",
            Self::FlyFast => "fly_fast",
        }
    }
}
