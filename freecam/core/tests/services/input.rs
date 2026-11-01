use freecam_core::services::InputTracker;
use freecam_core::structs::{FreecamAction, HeldKeys};

#[test]
fn flight_keybinds_set_and_clear_held() {
    let mut input = InputTracker::new();

    input.on_flight_key(FreecamAction::FlyForward, true);
    input.on_flight_key(FreecamAction::FlyFast, true);
    assert_eq!(input.held(), HeldKeys { forward: true, fast: true, ..HeldKeys::default() });

    input.on_flight_key(FreecamAction::FlyForward, false);
    assert_eq!(input.held(), HeldKeys { fast: true, ..HeldKeys::default() });
}

#[test]
fn every_flight_action_has_its_own_held_key() {
    let pairs = [
        (FreecamAction::FlyForward, HeldKeys { forward: true, ..HeldKeys::default() }),
        (FreecamAction::FlyBack, HeldKeys { back: true, ..HeldKeys::default() }),
        (FreecamAction::FlyLeft, HeldKeys { left: true, ..HeldKeys::default() }),
        (FreecamAction::FlyRight, HeldKeys { right: true, ..HeldKeys::default() }),
        (FreecamAction::FlyUp, HeldKeys { up: true, ..HeldKeys::default() }),
        (FreecamAction::FlyDown, HeldKeys { down: true, ..HeldKeys::default() }),
        (FreecamAction::FlyFast, HeldKeys { fast: true, ..HeldKeys::default() }),
    ];
    for (action, held) in pairs {
        let mut input = InputTracker::new();
        input.on_flight_key(action, true);
        assert_eq!(input.held(), held, "{action:?}");
    }
}

#[test]
fn other_actions_do_not_change_held() {
    let mut input = InputTracker::new();
    for action in [FreecamAction::Toggle, FreecamAction::AddPoint, FreecamAction::ToggleBar] {
        input.on_flight_key(action, true);
    }
    assert_eq!(input.held(), HeldKeys::default());
}

#[test]
fn pointer_look_only_when_active_and_no_menu() {
    let mut input = InputTracker::new();
    input.on_pointer_delta(5.0, 5.0, false, false);
    input.on_pointer_delta(5.0, 5.0, true, true);
    assert_eq!(input.take_look(), (0.0, 0.0));

    input.on_pointer_delta(3.0, -2.0, true, false);
    input.on_pointer_delta(1.5, 1.0, true, false);
    assert_eq!(input.take_look(), (4.5, -1.0));
    assert_eq!(input.take_look(), (0.0, 0.0));
}

#[test]
fn wheel_scrolls_one_step_per_event_by_its_sign() {
    let mut input = InputTracker::new();
    input.on_wheel(120.0, true, false);
    input.on_wheel(0.5, true, false);
    assert_eq!(input.take_scroll(), 2);
    input.on_wheel(-1.0, true, false);
    assert_eq!(input.take_scroll(), -1);
    input.on_wheel(0.0, true, false);
    assert_eq!(input.take_scroll(), 0);
}

#[test]
fn wheel_ignored_when_inactive_or_in_menu() {
    let mut input = InputTracker::new();
    input.on_wheel(1.0, false, false);
    input.on_wheel(1.0, true, true);
    assert_eq!(input.take_scroll(), 0);
}

#[test]
fn clear_held_releases_everything() {
    let mut input = InputTracker::new();
    input.on_flight_key(FreecamAction::FlyForward, true);
    input.on_flight_key(FreecamAction::FlyFast, true);

    input.clear_held();

    assert_eq!(input.held(), HeldKeys::default());
}
