use alaydriem_freecam::services::InputBridge;
use alaydriem_freecam::types::InputItem;
use amml_api::amml_abi::{AmmlMouseAction, AmmlMouseButton, MouseInput};
use amml_api::testing::FakeGame;

struct Fixture;

impl Fixture {
    fn mouse(action: AmmlMouseAction, notches: f32) -> MouseInput {
        MouseInput {
            header: FakeGame::header::<MouseInput>(),
            action,
            button: AmmlMouseButton::LEFT,
            down: 0,
            _reserved: 0,
            x: 0,
            y: 0,
            dx: 4,
            dy: 0,
            notches,
        }
    }

    fn bridge(game: &FakeGame, holding: bool, panel_open: bool) -> InputBridge {
        let bridge = InputBridge::new();
        bridge.set_gate(holding, panel_open, &game.ctx().log());
        bridge
    }

    fn drained(bridge: &InputBridge) -> Vec<InputItem> {
        let mut out = Vec::new();
        bridge.drain(&mut out);
        out
    }
}

#[test]
fn wheel_goes_to_freecam_and_not_to_the_game_while_the_gate_is_open() {
    let game = FakeGame::new();
    let bridge = Fixture::bridge(&game, true, false);
    let mut event = Fixture::mouse(AmmlMouseAction::WHEEL, 1.0);

    bridge.on_mouse(&mut event);

    assert_eq!(event.header.cancelled, 1);
    assert_eq!(Fixture::drained(&bridge), vec![InputItem::Wheel(1.0)]);
}

#[test]
fn wheel_reaches_the_game_while_freecam_holds_no_lease() {
    let game = FakeGame::new();
    let bridge = Fixture::bridge(&game, false, false);
    let mut event = Fixture::mouse(AmmlMouseAction::WHEEL, -1.0);

    bridge.on_mouse(&mut event);

    assert_eq!(event.header.cancelled, 0);
    assert!(Fixture::drained(&bridge).is_empty());
}

#[test]
fn wheel_is_left_to_the_manager_while_its_panel_is_open() {
    let game = FakeGame::new();
    let bridge = Fixture::bridge(&game, true, true);
    let mut event = Fixture::mouse(AmmlMouseAction::WHEEL, 1.0);

    bridge.on_mouse(&mut event);

    assert_eq!(event.header.cancelled, 0);
    assert!(Fixture::drained(&bridge).is_empty());
}

#[test]
fn a_mouse_move_is_not_cancelled_while_the_gate_is_open() {
    let game = FakeGame::new();
    let bridge = Fixture::bridge(&game, true, false);
    let mut event = Fixture::mouse(AmmlMouseAction::MOVE, 0.0);

    bridge.on_mouse(&mut event);

    assert_eq!(event.header.cancelled, 0);
    assert_eq!(Fixture::drained(&bridge), vec![InputItem::Look { dx: 4.0, dy: 0.0 }]);
}
