use alaydriem_freecam::services::{BindMatcher, InputQueue};
use alaydriem_freecam::types::InputItem;
use amm_core::structs::event::KeybindPhase;
use amml_api::{KeyCode, Modifiers};
use freecam_core::config::KeyConfig;
use freecam_core::structs::FreecamAction;

struct Rig {
    matcher: BindMatcher,
    queue: InputQueue,
}

impl Rig {
    // the default chords, with the lease held and the panel closed
    fn flying() -> Self {
        let mut rig = Self::idle();
        rig.matcher.set_gate(true, false, &rig.queue);
        rig
    }

    // the default chords, without the lease
    fn idle() -> Self {
        Self { matcher: BindMatcher::new(&KeyConfig::default()), queue: InputQueue::new(64) }
    }

    fn key(&mut self, key: KeyCode, down: bool, modifiers: Modifiers) {
        self.matcher.on_key(key, down, false, modifiers, &self.queue);
    }

    fn repeat(&mut self, key: KeyCode, modifiers: Modifiers) {
        self.matcher.on_key(key, true, true, modifiers, &self.queue);
    }

    fn binds(&self) -> Vec<(FreecamAction, KeybindPhase)> {
        let mut out = Vec::new();
        self.queue.drain(&mut out);
        out.into_iter()
            .map(|item| match item {
                InputItem::Bind(action, phase) => (action, phase),
                other => panic!("not a bind: {other:?}"),
            })
            .collect()
    }
}

#[test]
fn a_flight_key_under_the_lease_is_pressed_and_released() {
    let mut rig = Rig::flying();

    rig.key(KeyCode::W, true, Modifiers::NONE);
    rig.key(KeyCode::W, false, Modifiers::NONE);

    assert_eq!(
        rig.binds(),
        vec![(FreecamAction::FlyForward, KeybindPhase::Pressed), (FreecamAction::FlyForward, KeybindPhase::Released)]
    );
}

#[test]
fn keys_without_the_lease_match_nothing() {
    let mut rig = Rig::idle();

    rig.key(KeyCode::W, true, Modifiers::NONE);
    rig.key(KeyCode::P, true, Modifiers::F3);

    assert!(rig.binds().is_empty());
}

#[test]
fn an_open_panel_holds_back_presses() {
    let mut rig = Rig::flying();
    rig.matcher.set_gate(true, true, &rig.queue);

    rig.key(KeyCode::W, true, Modifiers::NONE);

    assert!(rig.binds().is_empty());
}

#[test]
fn an_opening_panel_releases_the_held_flight_keys() {
    let mut rig = Rig::flying();
    rig.key(KeyCode::W, true, Modifiers::NONE);
    rig.binds();

    rig.matcher.set_gate(true, true, &rig.queue);
    rig.key(KeyCode::W, false, Modifiers::NONE);

    assert_eq!(rig.binds(), vec![(FreecamAction::FlyForward, KeybindPhase::Released)]);
}

#[test]
fn a_lost_lease_releases_the_held_flight_keys() {
    let mut rig = Rig::flying();
    rig.key(KeyCode::A, true, Modifiers::NONE);
    rig.binds();

    rig.matcher.set_gate(false, false, &rig.queue);

    assert_eq!(rig.binds(), vec![(FreecamAction::FlyLeft, KeybindPhase::Released)]);
}

#[test]
fn f3_chords_match_with_the_f3_modifier_only() {
    let mut rig = Rig::flying();

    rig.key(KeyCode::P, true, Modifiers::NONE);
    rig.key(KeyCode::P, false, Modifiers::NONE);
    rig.key(KeyCode::P, true, Modifiers::F3);

    assert_eq!(rig.binds(), vec![(FreecamAction::Toggle, KeybindPhase::Pressed)]);
}

#[test]
fn ctrl_b_adds_a_point_and_holds_fly_fast() {
    let mut rig = Rig::flying();

    rig.key(KeyCode::LCONTROL, true, Modifiers::CTRL);
    rig.key(KeyCode::B, true, Modifiers::CTRL);

    assert_eq!(
        rig.binds(),
        vec![(FreecamAction::FlyFast, KeybindPhase::Pressed), (FreecamAction::AddPoint, KeybindPhase::Pressed)]
    );
}

#[test]
fn f3_o_orbits_and_plain_o_does_not() {
    let mut rig = Rig::flying();

    rig.key(KeyCode::O, true, Modifiers::NONE);
    rig.key(KeyCode::O, false, Modifiers::NONE);
    rig.key(KeyCode::O, true, Modifiers::F3);

    assert_eq!(rig.binds(), vec![(FreecamAction::Orbit, KeybindPhase::Pressed)]);
}

#[test]
fn a_flight_key_flies_with_other_modifiers_held() {
    let mut rig = Rig::flying();

    rig.key(KeyCode::W, true, Modifiers::CTRL | Modifiers::SHIFT);

    assert_eq!(rig.binds(), vec![(FreecamAction::FlyForward, KeybindPhase::Pressed)]);
}

#[test]
fn a_flight_key_repeat_with_no_seen_down_counts_as_a_press() {
    let mut rig = Rig::flying();

    rig.repeat(KeyCode::W, Modifiers::NONE);
    rig.repeat(KeyCode::W, Modifiers::NONE);

    assert_eq!(
        rig.binds(),
        vec![(FreecamAction::FlyForward, KeybindPhase::Pressed), (FreecamAction::FlyForward, KeybindPhase::Repeat)]
    );
}

#[test]
fn an_up_with_no_seen_down_gives_nothing() {
    let mut rig = Rig::flying();

    rig.key(KeyCode::P, false, Modifiers::F3);

    assert!(rig.binds().is_empty());
}

#[test]
fn a_rebind_during_flight_takes_effect_at_once() {
    let mut rig = Rig::flying();
    rig.key(KeyCode::W, true, Modifiers::NONE);
    rig.binds();
    let keys = KeyConfig { fly_forward: "I".to_owned(), ..KeyConfig::default() };

    rig.matcher.set_keys(&keys, &rig.queue);
    rig.key(KeyCode::W, true, Modifiers::NONE);
    rig.key(KeyCode::I, true, Modifiers::NONE);

    assert_eq!(
        rig.binds(),
        vec![(FreecamAction::FlyForward, KeybindPhase::Released), (FreecamAction::FlyForward, KeybindPhase::Pressed)]
    );
}

#[test]
fn a_chord_that_does_not_parse_matches_nothing() {
    let keys = KeyConfig { add_point: "Ctrl+NoSuchKey".to_owned(), ..KeyConfig::default() };
    let mut rig = Rig { matcher: BindMatcher::new(&keys), queue: InputQueue::new(8) };
    rig.matcher.set_gate(true, false, &rig.queue);

    rig.key(KeyCode::B, true, Modifiers::CTRL);

    assert!(rig.binds().is_empty());
}

#[test]
fn a_one_shot_repeat_with_no_seen_down_gives_nothing() {
    let mut rig = Rig::flying();

    rig.repeat(KeyCode::P, Modifiers::F3);
    rig.repeat(KeyCode::B, Modifiers::CTRL);

    assert!(rig.binds().is_empty());
}

#[test]
fn a_bind_held_when_the_gate_closes_lingers_until_its_key_goes_up() {
    let mut rig = Rig::flying();
    rig.key(KeyCode::P, true, Modifiers::F3);
    rig.binds();

    rig.matcher.set_gate(false, false, &rig.queue);
    let lingering = rig.matcher.lingering(FreecamAction::Toggle);
    rig.key(KeyCode::P, false, Modifiers::F3);

    assert!(lingering);
    assert!(!rig.matcher.lingering(FreecamAction::Toggle));
    assert!(!rig.matcher.lingering(FreecamAction::FlyForward));
}
