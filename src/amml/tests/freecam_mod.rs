use alaydriem_freecam::FreecamMod;
use amm_client::link::LinkUses;
use amml_api::amml_abi::AmmlOrigin;
use amml_api::testing::FakeGame;
use amml_api::{AmmlMod as _, BlockHit, BlockPos, Face, InstructionKind, Vec3};

use crate::support::FreecamRig;

#[test]
fn the_uses_block_has_every_row_the_manager_link_needs() {
    LinkUses::check(&FreecamMod::USES).unwrap();
}

#[test]
fn the_link_logs_once_the_manager_is_there() {
    let rig = FreecamRig::new();

    rig.frame();
    rig.frame();

    assert_eq!(rig.lines("info freecam linked").len(), 1);
}

#[test]
fn the_first_world_frame_logs_the_active_camera() {
    let rig = FreecamRig::new();

    rig.frames(3);

    assert_eq!(rig.lines("info camera watch kind=first_person").len(), 1);
}

#[test]
fn a_start_sets_the_free_camera_and_a_stop_releases_it() {
    let rig = FreecamRig::new();
    rig.frame();

    rig.toggle_on();
    assert!(rig.logged("info freecam on free_camera=set"), "{:?}", rig.game.logs());
    rig.toggle_off();

    let calls = rig.free_calls();
    assert!(calls.first().is_some_and(|call| call.starts_with("set_free 10,65.6,10")), "{calls:?}");
    assert_eq!(calls.last().map(String::as_str), Some("release_free"));
    assert!(rig.logged("info freecam off free_camera=released"));
}

#[test]
fn the_pose_is_set_in_every_frame_of_a_flight_and_not_after_it() {
    let rig = FreecamRig::new();
    rig.frame();

    rig.toggle_on();
    rig.frames(5);
    let flying = rig.camera.poses().len();
    rig.toggle_off();
    let after_exit = rig.camera.poses().len();
    rig.frames(3);

    assert!(flying >= 6, "{flying}");
    let first = rig.camera.poses()[0];
    assert_eq!((first.position.x, first.position.y, first.position.z), (10.0, 65.6, 10.0));
    assert_eq!(rig.camera.poses().len(), after_exit, "a pose after the flight ended");
    assert_eq!(rig.lines("info camera pose applied frames=1").len(), 1);
    assert!(rig.logged("info camera pose applied frames="), "{:?}", rig.game.logs());
    assert!(rig.lines("info camera pose applied").iter().any(|line| line.ends_with("flight=ended")));
}

#[test]
fn a_free_camera_not_ready_is_asked_again_until_the_handler_is_seen() {
    let rig = FreecamRig::without_handler();
    rig.frame();
    rig.toggle_on();
    rig.frames(3);
    let refused = rig.lines("info free camera call refused");

    rig.game.capture_handler();
    rig.frames(2);

    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].contains("error=the game is not running yet"), "{refused:?}");
    assert_eq!(rig.sets(), 2);
    assert_eq!(rig.lines("info freecam on free_camera=set").len(), 1);
}

#[test]
fn a_later_setup_of_the_same_frame_runs_no_second_core_frame() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    let before = rig.sets();

    rig.second_setup();

    assert_eq!(rig.sets(), before);
}

#[test]
fn a_flight_key_under_the_lease_moves_the_camera() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.game.real_key(FreecamRig::KEY_W, true);
    // core moves by the clock, so the frames of the hold must take real time
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        rig.frame();
    }
    rig.game.real_key(FreecamRig::KEY_W, false);
    rig.frame();

    let moved = rig.lines("info flight moved distance=");
    assert_eq!(moved.len(), 1, "{:?}", rig.game.logs());
    let distance: f32 = moved[0].rsplit('=').next().unwrap().parse().unwrap();
    assert!(distance > 0.0, "{distance}");
}

#[test]
fn a_server_camera_instruction_stops_the_flight() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.game.raise_camera_instruction(InstructionKind::Set, AmmlOrigin::GAME);
    rig.frames(2);
    let sets = rig.sets();
    rig.frames(3);

    assert!(rig.logged("info flight stopped reason=server_camera origin=game"));
    assert_eq!(rig.sets(), sets, "no set_free after the server took the camera");
}

#[test]
fn the_echo_of_its_own_free_camera_does_not_stop_the_flight() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    let sets = rig.sets();

    rig.game.raise_camera_instruction(InstructionKind::Set, AmmlOrigin::THIS_MOD);
    rig.frames(3);

    assert!(!rig.logged("info flight stopped"));
    assert_eq!(rig.sets(), sets + 3);
}

#[test]
fn a_health_drop_ends_the_flight() {
    let mut rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.set_client(|client| client.health = Some(19));
    rig.frames(30);

    assert_eq!(rig.free_calls().last().map(String::as_str), Some("release_free"));
}

#[test]
fn a_path_point_draws_markers_and_a_new_target_epoch_sends_them_again() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.add_point();
    rig.frames(2);
    let first = rig.game.shape_ops().len();
    rig.game.set_target_epoch(2);
    rig.frames(2);

    assert!(rig.logged("info path point added count=1"), "{:?}", rig.game.logs());
    assert!(first > 0);
    assert!(rig.logged("info markers applied delivered="));
    assert!(rig.game.shape_ops().len() >= 2 * first, "{} then {}", first, rig.game.shape_ops().len());
}

#[test]
fn a_look_at_ray_is_answered_from_the_block_hit() {
    let mut rig = FreecamRig::new();
    rig.set_ray_hit(Some(BlockHit {
        pos: BlockPos { x: 10, y: 60, z: 30 },
        point: Vec3 { x: 10.5, y: 61.0, z: 30.0 },
        face: Face::North,
    }));
    rig.frame();
    rig.toggle_on();

    rig.add_point();

    assert!(rig.logged("info look-at ray answered hit=block_or_actor"), "{:?}", rig.game.logs());
}

#[test]
fn binds_work_while_the_tracked_focus_is_lost() {
    let rig = FreecamRig::new();
    rig.focus(false);
    rig.frame();
    rig.toggle_on();
    rig.frames(2);
    let before = rig.free_calls().last().cloned().unwrap();

    rig.game.real_key(FreecamRig::KEY_W, true);
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        rig.frame();
    }
    let after = rig.free_calls().last().cloned().unwrap();
    rig.game.real_key(FreecamRig::KEY_W, false);
    rig.toggle_off();

    assert_ne!(before, after, "W did not fly with the tracked focus lost");
    assert!(rig.logged("info freecam off free_camera=released"), "F3+P did not turn freecam off");
}

#[test]
fn every_focus_loss_clears_the_flight_keys_also_when_focus_was_already_lost() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    rig.focus(false);
    rig.frames(2);

    rig.game.real_key(FreecamRig::KEY_W, true);
    rig.frames(2);
    rig.focus(false);
    rig.frames(2);

    assert_eq!(rig.lines("info flight keys cleared reason=focus_lost").len(), 2, "{:?}", rig.game.logs());
    // the held W was released by the loss, so the flight probe measured the move
    assert_eq!(rig.lines("info flight moved distance=").len(), 1, "{:?}", rig.game.logs());
}

#[test]
fn a_new_press_after_a_focus_loss_flies_again() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    rig.game.real_key(FreecamRig::KEY_W, true);
    rig.frames(2);
    rig.focus(false);
    rig.frames(2);

    rig.game.real_key(FreecamRig::KEY_W, false);
    rig.game.real_key(FreecamRig::KEY_W, true);
    rig.frames(2);
    rig.game.real_key(FreecamRig::KEY_W, false);
    rig.frame();

    assert_eq!(rig.lines("info flight moved distance=").len(), 2, "{:?}", rig.game.logs());
}

#[test]
fn a_manager_keybind_during_the_lease_is_dropped() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    let sets = rig.sets();

    // a manager toggle while freecam holds its lease would end the flight if it were not dropped
    amm_client::test_support::FakeTable::push_event(amm_core::structs::event::ModEvent::Keybind {
        action: 0,
        phase: amm_core::structs::event::KeybindPhase::Pressed,
    });
    rig.frames(3);

    assert_eq!(rig.sets(), sets + 3);
}

#[test]
fn a_disable_drops_the_driver_and_the_listeners() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.game.disable::<FreecamMod>();
    let sets = rig.sets();
    rig.frames(3);

    assert_eq!(rig.sets(), sets);
}

#[test]
fn a_world_leave_ends_the_flight_without_a_release() {
    let mut rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.world_left();
    rig.set_client(|client| {
        client.local_player = false;
        client.world_active = false;
    });
    rig.game.raise_with_client(&mut amml_api::amml_abi::ClientTick {
        header: FakeGame::header::<amml_api::amml_abi::ClientTick>(),
    });

    assert!(!rig.free_calls().iter().any(|call| call == "release_free"));
    let sets = rig.sets();
    rig.set_client(|client| {
        client.local_player = true;
        client.world_active = true;
    });
    rig.frames(3);
    assert_eq!(rig.sets(), sets, "the flight did not end at the world leave");
}

#[test]
fn a_focus_change_is_logged_once_per_change() {
    let rig = FreecamRig::new();
    rig.frame();

    rig.focus(false);
    rig.focus(false);
    rig.focus(true);

    assert_eq!(rig.lines("debug window focus focused=false").len(), 1, "{:?}", rig.game.logs());
    assert_eq!(rig.lines("debug window focus focused=true").len(), 1);
}

#[test]
fn the_bind_gate_logs_each_change_of_its_inputs() {
    let rig = FreecamRig::new();
    rig.frame();

    rig.toggle_on();
    rig.focus(false);
    rig.frames(2);
    rig.toggle_off();

    assert_eq!(rig.lines("debug bind gate open=true holding=true panel_open=false").len(), 1, "{:?}", rig.game.logs());
    // the first frame before the flight and the end of the flight
    assert_eq!(rig.lines("debug bind gate open=false holding=false panel_open=false").len(), 2);
}

#[test]
fn the_first_keys_under_the_lease_are_logged() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.game.real_key(FreecamRig::KEY_W, true);
    rig.game.real_key(FreecamRig::KEY_W, false);

    let seen = rig.lines("debug flight key seen");
    assert_eq!(seen.len(), 2, "{:?}", rig.game.logs());
    assert!(seen[0].starts_with("debug flight key seen key=0x57 down=1 repeat=0 cancelled="), "{seen:?}");
}

#[test]
fn a_flight_logs_that_its_world_frames_run() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();

    rig.frames(FreecamRig::HEARTBEAT_FRAMES);

    assert_eq!(rig.lines("debug world frames running count=").len(), 1, "{:?}", rig.game.logs());
}

#[test]
fn a_focus_loss_between_frames_clears_the_keys_when_a_client_tick_comes_first() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    rig.game.real_key(FreecamRig::KEY_W, true);
    rig.frames(2);

    rig.focus(true);
    rig.focus(false);
    rig.tick();
    rig.frame();

    assert_eq!(rig.lines("info flight keys cleared reason=focus_lost").len(), 1, "{:?}", rig.game.logs());
}

#[test]
fn a_held_toggle_chord_repeating_under_the_lease_does_not_turn_freecam_off() {
    let rig = FreecamRig::new();
    rig.frame();
    // F3+P goes down before the lease: the manager turns freecam on, the matcher sees no down
    rig.game.real_key(FreecamRig::KEY_F3, true);
    rig.game.real_key(FreecamRig::KEY_P, true);
    rig.toggle_on();
    let sets = rig.sets();

    // P held past the key-repeat delay: repeats of a key whose down the matcher never saw
    rig.game.real_key(FreecamRig::KEY_P, true);
    rig.game.real_key(FreecamRig::KEY_P, true);
    rig.frames(3);

    assert_eq!(rig.sets(), sets + 3, "freecam turned off on a repeat");
}

#[test]
fn a_held_toggle_chord_after_a_toggle_off_does_not_turn_freecam_on_again() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    rig.game.real_key(FreecamRig::KEY_F3, true);
    rig.game.real_key(FreecamRig::KEY_P, true);
    rig.frames(30);
    assert!(rig.logged("info freecam off free_camera=released"));

    // the manager counts a repeat of the blocked P it never saw down as a new press of its toggle
    amm_client::test_support::FakeTable::push_event(amm_core::structs::event::ModEvent::Keybind {
        action: 0,
        phase: amm_core::structs::event::KeybindPhase::Pressed,
    });
    rig.frames(3);
    let after_held = rig.lines("info freecam on free_camera=set").len();
    rig.game.real_key(FreecamRig::KEY_P, false);
    rig.game.real_key(FreecamRig::KEY_F3, false);
    rig.toggle_on();

    assert_eq!(after_held, 1, "the held chord turned freecam on again");
    assert_eq!(rig.lines("info freecam on free_camera=set").len(), 2, "a new press did not turn freecam on");
}

#[test]
fn the_mouse_does_not_turn_the_camera_while_the_panel_is_open() {
    let rig = FreecamRig::new();
    rig.frame();
    rig.toggle_on();
    amm_client::test_support::FakeTable::with(|state| state.panel_open = true);
    rig.frames(2);
    let before = rig.free_calls().last().cloned().unwrap();

    rig.mouse_move(400, 0);
    rig.mouse_wheel(3.0);
    rig.frames(3);
    let after = rig.free_calls().last().cloned().unwrap();
    amm_client::test_support::FakeTable::with(|state| state.panel_open = false);
    rig.frames(2);
    rig.mouse_move(400, 0);
    rig.frames(3);
    let turned = rig.free_calls().last().cloned().unwrap();

    assert_eq!(before, after, "the mouse turned the camera while the panel was open");
    assert_ne!(after, turned, "the mouse did not turn the camera after the panel closed");
}

#[test]
fn camera_instructions_past_the_bound_are_dropped_and_logged() {
    let rig = FreecamRig::new();
    rig.frame();

    for _ in 0..20 {
        rig.game.raise_camera_instruction(InstructionKind::Other, AmmlOrigin::GAME);
    }
    rig.frame();

    assert_eq!(rig.lines("warn camera instructions dropped count=4").len(), 1, "{:?}", rig.game.logs());
}
