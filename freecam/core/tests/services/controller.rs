use std::collections::HashMap;

use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::registry::ModIndex;
use freecam_core::config::{FreecamConfig, KeyConfig, LookAtConfig, MarkerConfig, MotionConfig};
use freecam_core::services::FreecamController;
use freecam_core::structs::{
    CameraCommand, CameraPose, FrameOutput, FrameStatus, FreecamAction, FreecamState, MarkerKind, MarkerShape, PathKey,
    RayRequest, ServerInstruction, StartLeases, UiAction, Vec3,
};

use crate::support::frames::Frames;

const RIGHT: u8 = 1;
const FRAME_STEP: f64 = 0.05;
const EXIT_WAIT: usize = 10;

struct Rig {
    controller: FreecamController,
    time: f64,
    // the shapes the client shows after every marker batch so far, by id
    world: HashMap<u64, MarkerShape>,
    // entry count of each frame's marker batch
    batch_sizes: Vec<usize>,
    // the shim's handler capture count sent in every status, 0 for no captured handler
    generation: u32,
}

impl Rig {
    // look-at keys off, so the add-point key stores a plain key at once
    fn new() -> Self {
        Self::with_config(Self::config(false))
    }

    // the default config, so the add-point key asks for a ray
    fn look_at() -> Self {
        Self::with_config(Self::config(true))
    }

    fn config(look_at: bool) -> FreecamConfig {
        FreecamConfig {
            look_at: LookAtConfig { enabled: look_at, ..LookAtConfig::default() },
            ..FreecamConfig::default()
        }
    }

    fn with_config(config: FreecamConfig) -> Self {
        Self {
            controller: FreecamController::new(config),
            time: 0.0,
            world: HashMap::new(),
            batch_sizes: Vec::new(),
            generation: 1,
        }
    }

    // applies the frame's marker batch the way the client does
    fn apply_markers(&mut self) {
        let batch = self.controller.marker_batch();
        self.batch_sizes.push(batch.len());
        for entry in batch {
            if entry.shape.kind == MarkerKind::Remove {
                self.world.remove(&entry.id);
            } else {
                self.world.insert(entry.id, entry.shape);
            }
        }
    }

    fn last_batch_size(&self) -> usize {
        self.batch_sizes.last().copied().unwrap_or(0)
    }

    fn status(&mut self) -> FrameStatus {
        self.time += FRAME_STEP;
        FrameStatus { handler_generation: self.generation, ..Frames::status(self.time) }
    }

    fn frame(&mut self) -> FrameOutput {
        let status = self.status();
        let output = self.controller.frame(&status);
        self.apply_markers();
        output
    }

    fn frame_with(&mut self, edit: impl FnOnce(&mut FrameStatus)) -> FrameOutput {
        let mut status = self.status();
        edit(&mut status);
        let output = self.controller.frame(&status);
        self.apply_markers();
        output
    }

    fn frames(&mut self, count: usize) -> Vec<FrameOutput> {
        (0..count).map(|_| self.frame()).collect()
    }

    // the manager's keybind event, then the lease hand-off the FFI handle runs for a start request
    fn keybind(&mut self, action: FreecamAction, phase: KeybindPhase) {
        self.controller.on_link_event(&ModEvent::Keybind { action: action.index(), phase });
        if self.controller.take_start_request() {
            self.controller.finish_start(StartLeases::Granted);
        }
    }

    // presses and releases the keybind
    fn tap(&mut self, action: FreecamAction) {
        self.keybind(action, KeybindPhase::Pressed);
        self.keybind(action, KeybindPhase::Released);
    }

    // presses or releases a held keybind
    fn hold(&mut self, action: FreecamAction, down: bool) {
        self.keybind(action, if down { KeybindPhase::Pressed } else { KeybindPhase::Released });
    }

    fn event(&mut self, event: ModEvent) {
        self.controller.on_link_event(&event);
    }

    // mouse movement while freecam holds Input
    fn look(&mut self, dx: f32, dy: f32) {
        self.event(ModEvent::PointerDelta { dx, dy });
    }

    fn wheel(&mut self, delta: f32) {
        self.event(ModEvent::Wheel { delta });
    }

    fn press_toggle(&mut self) {
        self.tap(FreecamAction::Toggle);
    }

    fn start(&mut self) -> FrameOutput {
        self.press_toggle();
        self.frame()
    }

    // toggles freecam off; the returned frame is exit frame 1
    fn begin_exit(&mut self) -> FrameOutput {
        self.press_toggle();
        self.frame()
    }

    // presses and releases the delete-point keybind
    fn press_delete(&mut self) {
        self.tap(FreecamAction::DeletePoint);
    }

    // freecam on with the smooth camera off, so the camera stops as soon as keys are released
    fn start_sharp(&mut self) -> CameraPose {
        let pose = self.start().render_override.unwrap();
        self.tap(FreecamAction::ToggleSmooth);
        pose
    }

    // walks forward at the scroll speed and returns the pose reached
    fn walk(&mut self, frames: usize) -> CameraPose {
        self.hold(FreecamAction::FlyForward, true);
        let mut pose = self.frame().render_override.unwrap();
        for _ in 1..frames {
            pose = self.frame().render_override.unwrap();
        }
        self.hold(FreecamAction::FlyForward, false);
        pose
    }

    // starts freecam and records two points `frames` walking frames apart
    fn build_path(&mut self, frames: usize) -> (CameraPose, CameraPose) {
        let first = self.start_sharp();
        self.tap(FreecamAction::AddPoint);
        let last = self.walk(frames);
        self.tap(FreecamAction::AddPoint);
        (first, last)
    }

    // the same as build_path but with the smooth camera left on
    fn build_smooth_path(&mut self, frames: usize) -> (CameraPose, CameraPose) {
        let first = Self::pose(&self.start());
        self.tap(FreecamAction::AddPoint);
        let last = self.walk(frames);
        self.tap(FreecamAction::AddPoint);
        (first, last)
    }

    // turns bar mode on when it is off, clicks Play on the bar and runs one frame
    fn play(&mut self) -> FrameOutput {
        self.bar_on();
        self.controller.apply_ui_action(UiAction::Play);
        self.frame()
    }

    // presses the bar key while a path plays, leaving Ctrl held, and runs one frame
    fn stop(&mut self) -> FrameOutput {
        self.cursor()
    }

    // turns bar mode on when it is off
    fn bar_on(&mut self) {
        if !self.controller.ui_snapshot().cursor_mode {
            self.cursor();
        }
    }

    // a bar click and one frame
    fn click(&mut self, action: UiAction) -> FrameOutput {
        self.controller.apply_ui_action(action);
        self.frame()
    }

    // presses the bar key, leaving Ctrl held, and runs one frame
    fn cursor(&mut self) -> FrameOutput {
        self.tap(FreecamAction::ToggleBar);
        self.frame()
    }

    // the right mouse button down or up
    fn right(&mut self, down: bool) {
        self.event(ModEvent::PointerButton { button: RIGHT, down, x: 0.0, y: 0.0 });
    }

    // a right press away from the UI and one frame, which starts a look drag in bar mode
    fn start_drag(&mut self) -> FrameOutput {
        self.right(true);
        self.frame()
    }

    // freecam on and the cursor mode on, with Ctrl held
    fn start_with_cursor(&mut self) -> FrameOutput {
        self.start();
        self.cursor()
    }

    // starts freecam with the smooth camera off and records three points 20 walking frames apart
    fn three_points(&mut self) -> [CameraPose; 3] {
        let first = self.start_sharp();
        self.tap(FreecamAction::AddPoint);
        let second = self.walk(20);
        self.tap(FreecamAction::AddPoint);
        let third = self.walk(20);
        self.tap(FreecamAction::AddPoint);
        [first, second, third]
    }

    fn point_distance(&self, index: usize) -> f32 {
        self.controller.path().curve().point_distance(index)
    }

    fn pose(out: &FrameOutput) -> CameraPose {
        out.render_override.unwrap()
    }

    fn gap(a: CameraPose, b: CameraPose) -> f32 {
        (a.position - b.position).length()
    }
}

#[test]
fn toggle_starts_with_set_hide_and_block() {
    let mut rig = Rig::new();
    let out = rig.start();
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(out.camera, CameraCommand::Set(Frames::start_pose()));
    assert!(out.leases_wanted);
    assert!(out.leases_wanted);
    assert_eq!(out.render_override, Some(Frames::start_pose()));
}

#[test]
fn refuses_when_not_ready() {
    let mut rig = Rig::new();
    rig.press_toggle();
    let out = rig.frame_with(|s| s.ready = false);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
}

#[test]
fn refuses_when_server_camera_active() {
    let mut rig = Rig::new();
    rig.press_toggle();
    let out = rig.frame_with(|s| s.player_camera_active = false);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
}

#[test]
fn refuses_when_menu_open() {
    let mut rig = Rig::new();
    rig.press_toggle();
    let out = rig.frame_with(|s| s.menu_open = true);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
}

#[test]
fn refuses_when_not_in_world() {
    let mut rig = Rig::new();
    rig.press_toggle();
    let out = rig.frame_with(|s| s.in_world = false);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
}

#[test]
fn exit_waits_then_clears() {
    let mut rig = Rig::new();
    rig.start();
    let mut exit = vec![rig.begin_exit()];
    exit.extend(rig.frames(EXIT_WAIT - 1));
    for out in &exit[..EXIT_WAIT - 1] {
        assert_eq!(out.camera, CameraCommand::None);
        assert_eq!(out.render_override, None);
        assert!(out.leases_wanted);
    }
    let last = exit[EXIT_WAIT - 1];
    assert_eq!(last.camera, CameraCommand::Clear);
    assert!(!last.leases_wanted);
    assert!(!last.leases_wanted);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(exit.iter().all(|out| !matches!(out.camera, CameraCommand::Set(_))));
}

#[test]
fn second_toggle_during_exit_is_ignored() {
    let mut rig = Rig::new();
    rig.start();
    let mut exit = vec![rig.begin_exit()];
    exit.extend(rig.frames(1));
    rig.press_toggle();
    exit.extend(rig.frames(EXIT_WAIT - 2));
    let clears = exit.iter().filter(|out| out.camera == CameraCommand::Clear).count();
    assert_eq!(clears, 1);
    assert_eq!(exit[EXIT_WAIT - 1].camera, CameraCommand::Clear);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(rig.frames(5).iter().all(|out| out.camera == CameraCommand::None));
}

#[test]
fn damage_starts_exit() {
    let mut rig = Rig::new();
    rig.start();
    let hit = rig.frame_with(|s| s.health = 19);
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    assert_eq!(hit.camera, CameraCommand::None);
    let rest = rig.frames(EXIT_WAIT - 1);
    assert_eq!(rest[EXIT_WAIT - 2].camera, CameraCommand::Clear);
    assert_eq!(rig.controller.state(), FreecamState::Off);
}

#[test]
fn damage_compares_to_previous_frame_health() {
    let mut rig = Rig::new();
    rig.press_toggle();
    rig.frame_with(|s| s.health = 10);
    rig.frame_with(|s| s.health = 18);
    assert_eq!(rig.controller.state(), FreecamState::On);
    rig.frame_with(|s| s.health = 17);
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
}

#[test]
fn dimension_change_starts_exit() {
    let mut rig = Rig::new();
    rig.start();
    let out = rig.frame_with(|s| s.dimension = 1);
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    assert_eq!(out.camera, CameraCommand::None);
    let rest = rig.frames(EXIT_WAIT - 1);
    assert_eq!(rest[EXIT_WAIT - 2].camera, CameraCommand::Clear);
}

#[test]
fn server_set_hands_over_without_clear() {
    let mut rig = Rig::new();
    rig.start();
    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);
    let out = rig.frame();
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert_eq!(out.render_override, None);
    assert!(!out.leases_wanted);
    assert!(!out.leases_wanted);
    assert!(rig.frames(20).iter().all(|out| out.camera == CameraCommand::None));
}

#[test]
fn server_set_during_exit_cancels_clear() {
    let mut rig = Rig::new();
    rig.start();
    rig.begin_exit();
    rig.frames(2);
    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);
    let out = rig.frame();
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
    assert!(!out.leases_wanted);
    assert!(rig.frames(20).iter().all(|out| out.camera == CameraCommand::None));
}

#[test]
fn server_instruction_while_off_does_not_affect_next_start() {
    let mut rig = Rig::new();
    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);
    let out = rig.start();
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert!(matches!(out.camera, CameraCommand::Set(_)));
}

#[test]
fn server_other_ignored() {
    let mut rig = Rig::new();
    rig.start();
    rig.controller.on_server_instruction(ServerInstruction::Other, false);
    rig.frame();
    assert_eq!(rig.controller.state(), FreecamState::On);
}

#[test]
fn leaving_world_turns_off_without_clear() {
    let mut rig = Rig::new();
    rig.start();
    let out = rig.frame_with(|s| s.in_world = false);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(!out.leases_wanted);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
}

#[test]
fn menu_open_holds_camera() {
    let mut rig = Rig::new();
    rig.start();
    rig.hold(FreecamAction::FlyForward, true);
    let out = rig.frame_with(|s| s.menu_open = true);
    assert_eq!(out.camera, CameraCommand::Set(Frames::start_pose()));
    assert_eq!(out.render_override, Some(Frames::start_pose()));
    assert!(out.leases_wanted);
}

#[test]
fn held_key_moves_camera() {
    let mut rig = Rig::new();
    rig.start();
    rig.hold(FreecamAction::FlyForward, true);
    rig.frame();
    let out = rig.frame();
    let moved = out.render_override.unwrap().position;
    assert_ne!(moved, Frames::start_pose().position);
}

#[test]
fn focus_loss_clears_held_keys() {
    let mut rig = Rig::new();
    rig.start();
    rig.tap(FreecamAction::ToggleSmooth);
    rig.hold(FreecamAction::FlyForward, true);
    rig.frame_with(|s| s.focused = false);
    let positions: Vec<_> = rig.frames(3).iter().map(|out| out.render_override.unwrap().position).collect();
    assert_eq!(positions[0], positions[1]);
    assert_eq!(positions[1], positions[2]);
}

#[test]
fn chord_in_menu_does_not_toggle() {
    let mut rig = Rig::new();
    rig.frame_with(|s| s.menu_open = true);
    rig.press_toggle();
    let out = rig.frame();
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
}

#[test]
fn input_during_exit_does_not_leak_into_next_session() {
    let mut leaky = Rig::new();
    leaky.start();
    leaky.begin_exit();
    leaky.look(400.0, -300.0);
    leaky.wheel(1.0);
    leaky.frames(EXIT_WAIT - 1);
    assert_eq!(leaky.controller.state(), FreecamState::Off);
    let first = leaky.start();
    let start = Frames::start_pose();
    let pose = first.render_override.unwrap();
    assert_eq!((pose.yaw, pose.pitch), (start.yaw, start.pitch));
    let on_frame = leaky.frame().render_override.unwrap();
    assert_eq!((on_frame.yaw, on_frame.pitch), (start.yaw, start.pitch));

    let mut clean = Rig::new();
    clean.start();
    clean.begin_exit();
    clean.frames(EXIT_WAIT - 1);
    clean.start();
    clean.frame();
    leaky.hold(FreecamAction::FlyForward, true);
    clean.hold(FreecamAction::FlyForward, true);
    let leaky_pose = leaky.frame().render_override.unwrap();
    let clean_pose = clean.frame().render_override.unwrap();
    assert_eq!(leaky_pose.position, clean_pose.position);
}

#[test]
fn leaving_world_while_exiting_turns_off_without_clear() {
    let mut rig = Rig::new();
    rig.start();
    rig.begin_exit();
    let out = rig.frame_with(|s| s.in_world = false);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(!out.leases_wanted);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(rig.frames(20).iter().all(|out| out.camera == CameraCommand::None));
}

#[test]
fn del_key_without_selection_does_nothing() {
    let mut rig = Rig::new();
    let first = rig.start_sharp();
    rig.press_delete();
    assert!(rig.controller.path().points().is_empty());
    rig.tap(FreecamAction::AddPoint);
    rig.press_delete();
    assert_eq!(rig.controller.path().points(), &[first]);
    let second = rig.walk(10);
    rig.tap(FreecamAction::AddPoint);
    let third = rig.walk(10);
    rig.tap(FreecamAction::AddPoint);
    rig.press_delete();
    assert_eq!(rig.controller.path().points(), &[first, second, third]);
}

#[test]
fn play_with_fewer_than_two_points_does_nothing() {
    let mut rig = Rig::new();
    rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    let here = rig.walk(5);
    let out = rig.play();
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(Rig::pose(&out), here);
    assert_eq!(Rig::pose(&rig.frame()), here);
}

#[test]
fn play_snaps_to_first_point_then_settles_then_moves() {
    let mut rig = Rig::new();
    let (first, last) = rig.build_path(40);
    let mut settle = vec![rig.play()];
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    settle.extend(rig.frames(EXIT_WAIT - 1));
    for out in &settle {
        assert_eq!(out.camera, CameraCommand::Set(first));
        assert_eq!(out.render_override, Some(first));
        assert!(out.leases_wanted);
    }
    let moving = Rig::pose(&rig.frame());
    assert!(Rig::gap(moving, first) > 0.0);
    assert!(Rig::gap(moving, last) < Rig::gap(first, last));
}

#[test]
fn play_uses_scroll_speed_not_fast_multiplier() {
    let mut rig = Rig::new();
    rig.build_path(80);
    rig.play();
    rig.frames(EXIT_WAIT - 1 + 30);
    let before = Rig::pose(&rig.frame());
    let after = Rig::pose(&rig.frame());
    let cruise_step = 6.0 * FRAME_STEP as f32;
    assert!((Rig::gap(before, after) - cruise_step).abs() < 0.01, "step {}", Rig::gap(before, after));
}

#[test]
fn play_finishes_and_holds_last_point() {
    let mut rig = Rig::new();
    let (_, last) = rig.build_path(20);
    rig.play();
    let outs = rig.frames(EXIT_WAIT - 1 + 45);
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(Rig::pose(&outs[outs.len() - 1]), last);
    for out in rig.frames(3) {
        assert_eq!(Rig::pose(&out), last);
    }
}

#[test]
fn toggle_during_play_stops_in_place() {
    let mut rig = Rig::new();
    rig.build_path(80);
    rig.play();
    rig.frames(EXIT_WAIT - 1 + 20);
    let here = Rig::pose(&rig.frame());
    let stopped = rig.stop();
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(Rig::pose(&stopped), here);
    for out in rig.frames(3) {
        assert_eq!(Rig::pose(&out), here);
    }
}

#[test]
fn delete_key_during_playback_changes_nothing() {
    let mut rig = Rig::new();
    let (first, last) = rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    rig.tap(FreecamAction::AddPoint);
    rig.press_delete();
    rig.frame();
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert_eq!(rig.controller.path().points(), &[first, last]);
}

#[test]
fn menu_pauses_playback() {
    let mut rig = Rig::new();
    rig.build_path(80);
    rig.play();
    rig.frames(EXIT_WAIT + 20);
    let before = Rig::pose(&rig.frame());
    for _ in 0..6 {
        let out = rig.frame_with(|s| s.menu_open = true);
        assert_eq!(Rig::pose(&out), before);
    }
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    let resumed = Rig::pose(&rig.frame());
    assert!(Rig::gap(before, resumed) < 0.5);
}

#[test]
fn look_input_ignored_while_playing() {
    let mut noisy = Rig::new();
    let mut quiet = Rig::new();
    for rig in [&mut noisy, &mut quiet] {
        rig.build_path(80);
        rig.play();
        rig.frames(EXIT_WAIT + 10);
    }
    noisy.look(400.0, -300.0);
    noisy.wheel(1.0);
    noisy.hold(FreecamAction::FlyForward, true);
    assert_eq!(noisy.frame().render_override, quiet.frame().render_override);
    noisy.hold(FreecamAction::FlyForward, false);
    for rig in [&mut noisy, &mut quiet] {
        rig.stop();
    }
    assert_eq!(noisy.controller.state(), FreecamState::On);
    for _ in 0..3 {
        assert_eq!(noisy.frame().render_override, quiet.frame().render_override);
    }
}

#[test]
fn damage_during_playback_exits() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    let hit = rig.frame_with(|s| s.health = 19);
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    assert_eq!(hit.camera, CameraCommand::None);
    let rest = rig.frames(EXIT_WAIT - 1);
    assert_eq!(rest[EXIT_WAIT - 2].camera, CameraCommand::Clear);
    assert_eq!(rig.controller.state(), FreecamState::Off);
}

#[test]
fn server_set_during_playback_hands_over() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);
    let out = rig.frame();
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
    assert!(rig.frames(20).iter().all(|out| out.camera == CameraCommand::None));
}

#[test]
fn ctrl_p_during_playback_exits_with_clear() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    let mut exit = vec![rig.begin_exit()];
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    exit.extend(rig.frames(EXIT_WAIT - 1));
    assert_eq!(exit[EXIT_WAIT - 1].camera, CameraCommand::Clear);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    let restarted = rig.start();
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(restarted.camera, CameraCommand::Set(Frames::start_pose()));
    rig.frames(3);
    assert_eq!(rig.controller.state(), FreecamState::On);
}

#[test]
fn path_survives_freecam_off_on() {
    let mut rig = Rig::new();
    let (first, last) = rig.build_path(20);
    rig.begin_exit();
    rig.frames(EXIT_WAIT - 1);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    rig.start();
    assert_eq!(rig.controller.path().points(), &[first, last]);
    let out = rig.play();
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert_eq!(Rig::pose(&out), first);
}

#[test]
fn path_cleared_on_dimension_change_while_off() {
    let mut rig = Rig::new();
    rig.build_path(20);
    rig.begin_exit();
    rig.frames(EXIT_WAIT - 1);
    rig.frames(3);
    assert_eq!(rig.controller.path().points().len(), 2);
    rig.frame_with(|s| s.dimension = 1);
    assert!(rig.controller.path().points().is_empty());
}

#[test]
fn path_cleared_on_leaving_world() {
    let mut off = Rig::new();
    off.build_path(20);
    off.begin_exit();
    off.frames(EXIT_WAIT - 1);
    off.frame_with(|s| s.in_world = false);
    assert!(off.controller.path().points().is_empty());

    let mut on = Rig::new();
    on.build_path(20);
    on.frame_with(|s| s.in_world = false);
    assert_eq!(on.controller.state(), FreecamState::Off);
    assert!(on.controller.path().points().is_empty());
    on.start();
    on.play();
    assert_eq!(on.controller.state(), FreecamState::On);
}

#[test]
fn smooth_stop_leaves_camera_still() {
    let mut rig = Rig::new();
    rig.build_smooth_path(80);
    rig.play();
    rig.frames(EXIT_WAIT - 1 + 20);
    let here = Rig::pose(&rig.frame());
    let stopped = rig.stop();
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(Rig::pose(&stopped), here);
    for out in rig.frames(5) {
        assert_eq!(Rig::pose(&out), here);
    }
}

#[test]
fn smooth_finish_leaves_camera_still_on_last_point() {
    let mut rig = Rig::new();
    let (_, last) = rig.build_smooth_path(20);
    rig.play();
    let outs = rig.frames(EXIT_WAIT - 1 + 60);
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(Rig::pose(&outs[outs.len() - 1]), last);
    for out in rig.frames(5) {
        assert_eq!(Rig::pose(&out), last);
    }
}

#[test]
fn leaving_world_during_playback_turns_off_without_clear() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    let out = rig.frame_with(|s| s.in_world = false);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::None);
    assert!(!out.leases_wanted);
    assert!(!out.leases_wanted);
    assert!(rig.controller.path().points().is_empty());
    assert!(rig.frames(20).iter().all(|out| out.camera == CameraCommand::None));
}

#[test]
fn dimension_change_during_playback_exits_and_clears_path() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    let mut exit = vec![rig.frame_with(|s| s.dimension = 1)];
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    assert!(rig.controller.path().points().is_empty());
    for _ in 1..EXIT_WAIT {
        exit.push(rig.frame_with(|s| s.dimension = 1));
    }
    for out in &exit[..EXIT_WAIT - 1] {
        assert_eq!(out.camera, CameraCommand::None);
    }
    assert_eq!(exit[EXIT_WAIT - 1].camera, CameraCommand::Clear);
    assert!(!exit[EXIT_WAIT - 1].leases_wanted);
    assert_eq!(rig.controller.state(), FreecamState::Off);
}

#[test]
fn death_during_playback_waits_then_clears() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    let mut exit = vec![rig.frame_with(|s| s.health = 0)];
    for _ in 1..EXIT_WAIT {
        exit.push(rig.frame_with(|s| s.health = 0));
    }
    for out in &exit[..EXIT_WAIT - 1] {
        assert_eq!(out.camera, CameraCommand::None);
        assert!(out.leases_wanted);
    }
    assert_eq!(exit[EXIT_WAIT - 1].camera, CameraCommand::Clear);
    assert!(!exit[EXIT_WAIT - 1].leases_wanted);
    assert_eq!(rig.controller.state(), FreecamState::Off);
}

#[test]
fn ctrl_p_during_playback_sends_nothing_until_the_clear() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    let mut exit = vec![rig.begin_exit()];
    exit.extend(rig.frames(EXIT_WAIT - 1));
    for out in &exit[..EXIT_WAIT - 1] {
        assert_eq!(out.camera, CameraCommand::None);
        assert_eq!(out.render_override, None);
        assert!(out.leases_wanted);
    }
    assert_eq!(exit[EXIT_WAIT - 1].camera, CameraCommand::Clear);
    assert!(!exit[EXIT_WAIT - 1].leases_wanted);
}

#[test]
fn ctrl_e_turns_cursor_mode_on_and_releases_mouse() {
    let mut rig = Rig::new();
    rig.start();

    let out = rig.cursor();

    assert!(out.cursor_mode);
    assert!(out.ui_visible);
}

#[test]
fn cursor_mode_holds_without_repeating_release() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.frame();

    assert!(out.cursor_mode);
}

#[test]
fn ctrl_e_again_grabs_mouse() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.cursor();

    assert!(!out.cursor_mode);
}

#[test]
fn cursor_mode_needs_freecam() {
    let mut rig = Rig::new();

    let out = rig.cursor();

    assert!(!out.cursor_mode);
}

#[test]
fn mouse_move_in_cursor_mode_does_not_turn_camera() {
    let mut rig = Rig::new();
    let before = Rig::pose(&rig.start());
    rig.cursor();

    rig.look(500.0, 0.0);
    let after = Rig::pose(&rig.frame());

    assert_eq!(after.yaw, before.yaw);
}

#[test]
fn movement_while_on_is_discarded_when_it_ends() {
    let mut rig = Rig::new();
    let before = Rig::pose(&rig.start());
    rig.cursor();
    rig.look(500.0, 300.0);

    let out = rig.cursor();

    assert!(!out.cursor_mode);
    assert_eq!(Rig::pose(&out).yaw, before.yaw);
    assert_eq!(Rig::pose(&rig.frame()).yaw, before.yaw);
}

#[test]
fn menu_ends_cursor_mode_without_grab() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.frame_with(|status| status.menu_open = true);

    assert!(!out.cursor_mode);
    assert!(!rig.frame().cursor_mode);
}

#[test]
fn focus_loss_ends_cursor_mode_without_grab() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.frame_with(|status| status.focused = false);

    assert!(!out.cursor_mode);
}

#[test]
fn cursor_mode_does_not_start_while_unfocused() {
    let mut rig = Rig::new();
    rig.start();
    rig.tap(FreecamAction::ToggleBar);

    let out = rig.frame_with(|status| status.focused = false);

    assert!(!out.cursor_mode);
}

#[test]
fn exit_grabs_mouse_when_in_play() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.begin_exit();

    assert!(!out.cursor_mode);
}

#[test]
fn leaving_world_ends_cursor_mode_without_grab() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.frame_with(|status| {
        status.in_world = false;
        status.dimension = 1;
    });

    assert!(!out.cursor_mode);
}

#[test]
fn ctrl_e_during_playback_stops_and_keeps_bar_mode() {
    let mut rig = Rig::new();
    rig.build_path(40);
    rig.play();
    rig.frames(EXIT_WAIT + 5);
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    let here = Rig::pose(&rig.frame());

    let out = rig.stop();

    assert_eq!(rig.controller.state(), FreecamState::On);
    assert!(out.cursor_mode);
    assert!(out.ui_visible);
    assert_eq!(Rig::pose(&out), here);
    assert!(rig.controller.ui_snapshot().playhead > 0.0);
}

#[test]
fn playback_start_ends_cursor_mode() {
    let mut rig = Rig::new();
    rig.build_path(4);
    rig.cursor();

    let out = rig.play();

    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert!(!out.cursor_mode);
}

#[test]
fn right_drag_in_bar_mode_turns_camera() {
    let mut look = Rig::new();
    look.start();
    look.frames(2);
    let mut drag = Rig::new();
    drag.start();
    drag.cursor();
    let before = drag.frame().render_override.unwrap();

    drag.right(true);
    drag.look(300.0, 0.0);
    let grab = drag.frame();

    assert!(!grab.cursor_mode);
    assert!(drag.controller.ui_snapshot().cursor_mode);
    assert_eq!(Rig::pose(&grab).yaw, before.yaw);
    look.frame();
    look.look(200.0, 50.0);
    drag.look(200.0, 50.0);
    for _ in 0..5 {
        let expected = Rig::pose(&look.frame());
        let out = drag.frame();
        assert_eq!(Rig::pose(&out), expected);
        assert!(!out.cursor_mode);
    }
    assert_ne!(Rig::pose(&look.frame()).yaw, before.yaw);
}

#[test]
fn right_press_over_ui_does_not_drag() {
    let mut rig = Rig::new();
    let before = Rig::pose(&rig.start());
    rig.cursor();

    rig.controller.set_pointer_over_ui(true);
    rig.right(true);
    let out = rig.frame();
    rig.look(500.0, 0.0);
    rig.controller.set_pointer_over_ui(false);
    let later = rig.frame();

    assert!(out.cursor_mode);
    assert!(later.cursor_mode);
    assert_eq!(Rig::pose(&later).yaw, before.yaw);
}

#[test]
fn drag_ends_on_release_without_jump() {
    let mut rig = Rig::new();
    rig.start_sharp();
    rig.cursor();
    rig.start_drag();
    rig.look(200.0, 0.0);
    let turned = Rig::pose(&rig.frame());

    rig.right(false);
    let out = rig.frame();
    rig.look(500.0, 0.0);
    let after = rig.frame();

    assert!(out.cursor_mode);
    assert_eq!(Rig::pose(&out).yaw, turned.yaw);
    assert_eq!(Rig::pose(&after).yaw, turned.yaw);
}

#[test]
fn drag_ends_on_focus_loss_and_bar_off() {
    let mut unfocused = Rig::new();
    unfocused.start();
    unfocused.cursor();
    unfocused.start_drag();

    let lost = unfocused.frame_with(|status| status.focused = false);
    unfocused.right(false);
    let back = unfocused.frame();

    assert!(!lost.cursor_mode);
    assert!(!back.cursor_mode);

    let mut bar_off = Rig::new();
    bar_off.start();
    bar_off.cursor();
    bar_off.start_drag();

    let off = bar_off.cursor();
    let on_again = bar_off.cursor();
    let held = bar_off.frame();

    assert!(!off.cursor_mode);
    assert!(on_again.cursor_mode);
    assert!(held.cursor_mode);
}

#[test]
fn drag_ends_on_menu_playback_and_exit() {
    let mut menu = Rig::new();
    menu.start();
    menu.cursor();
    menu.start_drag();
    let out = menu.frame_with(|status| status.menu_open = true);
    assert!(!out.cursor_mode);
    menu.frame();
    assert!(menu.cursor().cursor_mode);

    let mut playback = Rig::new();
    playback.build_path(4);
    playback.cursor();
    playback.start_drag();
    let out = playback.play();
    assert_eq!(playback.controller.state(), FreecamState::Playing);
    assert!(!out.cursor_mode);

    let mut exit = Rig::new();
    exit.start();
    exit.cursor();
    exit.start_drag();
    let out = exit.begin_exit();
    assert!(!out.cursor_mode);
}

#[test]
fn right_press_outside_bar_mode_does_not_drag() {
    let mut rig = Rig::new();
    rig.start();

    rig.right(true);
    let out = rig.frame();
    let bar = rig.cursor();
    rig.right(false);

    assert!(!out.cursor_mode);
    assert!(bar.cursor_mode);
}

#[test]
fn wasd_still_flies_during_drag() {
    let mut rig = Rig::new();
    let start = rig.start_sharp();
    rig.cursor();
    rig.start_drag();

    rig.hold(FreecamAction::FlyForward, true);
    rig.look(100.0, 0.0);
    rig.frame();
    let out = rig.frame();

    assert!(!out.cursor_mode);
    assert_ne!(Rig::pose(&out).position, start.position);
    assert_ne!(Rig::pose(&out).yaw, start.yaw);
}

#[test]
fn ui_visible_with_zero_points() {
    let mut rig = Rig::new();
    let out = rig.start();

    assert!(out.ui_visible);
    assert!(rig.controller.ui_snapshot().visible);
    assert_eq!(rig.controller.ui_snapshot().point_count, 0);
}

#[test]
fn ui_hidden_when_freecam_is_off() {
    let mut rig = Rig::new();
    let out = rig.frame();

    assert!(!out.ui_visible);
    assert!(!rig.controller.ui_snapshot().visible);
}

#[test]
fn ui_hidden_under_menu_and_during_playback() {
    let mut rig = Rig::new();
    rig.build_path(4);

    let menu = rig.frame_with(|status| status.menu_open = true);
    assert!(!menu.ui_visible);
    assert!(!rig.controller.ui_snapshot().visible);

    rig.frame();
    rig.play();
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert!(!rig.controller.ui_snapshot().visible);
}

#[test]
fn snapshot_reports_points_and_total_seconds() {
    let mut rig = Rig::new();
    rig.build_path(20);

    let snapshot = rig.controller.ui_snapshot();

    assert_eq!(snapshot.point_count, 2);
    assert!(snapshot.total_seconds > 0.0);
}

#[test]
fn snapshot_reports_cursor_mode() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    assert!(rig.controller.ui_snapshot().cursor_mode);
}

#[test]
fn legend_follows_the_key_config() {
    let rig = Rig::new();

    assert_eq!(rig.controller.legend().rows.len(), 5);
}

#[test]
fn dimension_change_in_world_ends_cursor_mode() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.frame_with(|status| status.dimension = 1);

    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    assert!(!out.cursor_mode);
}

#[test]
fn damage_ends_cursor_mode() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    let out = rig.frame_with(|status| status.health = 19);

    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
    assert!(!out.cursor_mode);
}

#[test]
fn server_handover_ends_cursor_mode() {
    let mut rig = Rig::new();
    rig.start_with_cursor();
    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);

    let out = rig.frame();

    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(!out.cursor_mode);
}

#[test]
fn wheel_over_the_ui_in_cursor_mode_does_not_change_speed() {
    let mut rig = Rig::new();
    rig.build_path(20);
    let before = rig.controller.ui_snapshot().total_seconds;
    rig.cursor();

    rig.controller.set_pointer_over_ui(true);
    rig.wheel(1.0);
    rig.frame();

    assert!(rig.controller.ui_snapshot().cursor_mode);
    assert_eq!(rig.controller.ui_snapshot().total_seconds, before);
}

#[test]
fn wheel_off_the_ui_in_cursor_mode_changes_speed() {
    let mut rig = Rig::new();
    rig.build_path(20);
    let before = rig.controller.ui_snapshot().total_seconds;
    rig.cursor();

    rig.controller.set_pointer_over_ui(false);
    rig.wheel(1.0);
    rig.frame();

    assert!(rig.controller.ui_snapshot().cursor_mode);
    assert_ne!(rig.controller.ui_snapshot().total_seconds, before);
}

#[test]
fn wheel_outside_cursor_mode_changes_speed() {
    let mut rig = Rig::new();
    rig.build_path(20);
    let before = rig.controller.ui_snapshot().total_seconds;

    rig.wheel(1.0);
    rig.frame();

    assert_ne!(rig.controller.ui_snapshot().total_seconds, before);
}

#[test]
fn wheel_notches_over_the_ui_do_not_leak_after_cursor_mode_ends() {
    let mut rig = Rig::new();
    rig.build_path(20);
    let before = rig.controller.ui_snapshot().total_seconds;
    rig.cursor();
    rig.controller.set_pointer_over_ui(true);
    rig.wheel(1.0);
    rig.controller.set_pointer_over_ui(false);

    rig.cursor();
    rig.frame();

    assert_eq!(rig.controller.ui_snapshot().total_seconds, before);
}

// a rig with two points and the cursor mode on
fn cursor_rig_with_points() -> Rig {
    let mut rig = Rig::new();
    rig.build_path(4);
    rig.cursor();
    rig
}

// runs frames until `seconds` have passed
fn run_for(rig: &mut Rig, seconds: f64) {
    rig.frames((seconds / FRAME_STEP).ceil() as usize);
}

#[test]
fn clear_needs_two_clicks_within_arm_time() {
    let mut rig = cursor_rig_with_points();
    assert!(!rig.controller.ui_snapshot().clear_armed);

    rig.controller.apply_ui_action(UiAction::ClearPoints);
    assert!(rig.controller.ui_snapshot().clear_armed);
    assert_eq!(rig.controller.path().points().len(), 2);

    run_for(&mut rig, 2.0);
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    assert!(rig.controller.path().points().is_empty());
    assert!(!rig.controller.ui_snapshot().clear_armed);
}

#[test]
fn clear_arm_expires_after_three_seconds() {
    let mut rig = cursor_rig_with_points();
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    run_for(&mut rig, 3.2);
    assert!(!rig.controller.ui_snapshot().clear_armed);
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    assert_eq!(rig.controller.path().points().len(), 2);
    assert!(rig.controller.ui_snapshot().clear_armed);
}

#[test]
fn clear_disarms_when_cursor_mode_ends() {
    let mut rig = cursor_rig_with_points();
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    rig.cursor();
    assert!(!rig.controller.ui_snapshot().cursor_mode);
    assert!(!rig.controller.ui_snapshot().clear_armed);
    rig.cursor();
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    assert_eq!(rig.controller.path().points().len(), 2);
}

#[test]
fn clear_disarms_when_ui_hides() {
    let mut rig = cursor_rig_with_points();
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    rig.frame_with(|status| status.menu_open = true);
    assert!(!rig.controller.ui_snapshot().clear_armed);
    rig.frame();
    rig.cursor();
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    assert_eq!(rig.controller.path().points().len(), 2);
}

#[test]
fn clear_with_no_points_does_nothing() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    rig.controller.apply_ui_action(UiAction::ClearPoints);

    assert!(!rig.controller.ui_snapshot().clear_armed);
    rig.controller.apply_ui_action(UiAction::ClearPoints);
    assert!(rig.controller.path().points().is_empty());
}

#[test]
fn clear_outside_cursor_mode_does_nothing() {
    let mut rig = Rig::new();
    rig.build_path(4);

    rig.controller.apply_ui_action(UiAction::ClearPoints);
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    assert_eq!(rig.controller.path().points().len(), 2);
}

#[test]
fn camera_not_owned_while_off() {
    let mut rig = Rig::new();
    assert!(rig.frames(3).iter().all(|out| !out.camera_owned));
}

#[test]
fn camera_owned_after_toggle_on() {
    let mut rig = Rig::new();
    assert!(rig.start().camera_owned);
    assert!(rig.frames(3).iter().all(|out| out.camera_owned));
}

#[test]
fn camera_owned_during_playback() {
    let mut rig = Rig::new();
    rig.build_path(40);
    assert!(rig.play().camera_owned);
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert!(rig.frames(5).iter().all(|out| out.camera_owned));
}

#[test]
fn camera_owned_through_exit_wait_until_off() {
    let mut rig = Rig::new();
    rig.start();
    assert!(rig.begin_exit().camera_owned);
    let rest = rig.frames(EXIT_WAIT - 1);
    assert!(rest[..EXIT_WAIT - 2].iter().all(|out| out.camera_owned));
    assert_eq!(rest[EXIT_WAIT - 2].camera, CameraCommand::Clear);
    assert!(!rest[EXIT_WAIT - 2].camera_owned);
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(!rig.frame().camera_owned);
}

#[test]
fn camera_not_owned_after_forced_exit() {
    let mut rig = Rig::new();
    rig.start();
    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);
    assert!(!rig.frame().camera_owned);
    assert!(!rig.frame().camera_owned);
}

// one way for freecam to leave editing
type Exit = fn(&mut Rig);

// a rig with a two-point path whose markers have all been sent
fn marker_rig() -> Rig {
    let mut rig = Rig::new();
    rig.build_path(20);
    rig.frames(3);
    rig
}

#[test]
fn markers_show_when_freecam_on_with_points() {
    let rig = marker_rig();
    let shapes: Vec<&MarkerShape> = rig.world.values().collect();
    assert!(shapes.iter().any(|shape| shape.kind == MarkerKind::Arrow));
    assert!(shapes.iter().any(|shape| shape.kind == MarkerKind::Line && shape.color == 0xFF4B_E35A));
    assert!(rig.batch_sizes.iter().all(|size| *size <= MarkerConfig::default().max_per_send));
    assert_eq!(rig.last_batch_size(), 0);
}

#[test]
fn markers_hide_on_playback_start_and_return_after() {
    let mut rig = marker_rig();
    let shown = rig.world.clone();
    rig.play();
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert!(rig.world.is_empty());
    rig.frames(EXIT_WAIT - 1);
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert!(rig.world.is_empty());
    rig.frames(50);
    assert_eq!(rig.controller.state(), FreecamState::On);
    rig.frames(2);
    assert_eq!(rig.world, shown);
}

#[test]
fn markers_hide_on_freecam_off_and_each_forced_exit() {
    let exits: [(&str, Exit); 4] = [
        ("toggle", |rig| {
            rig.begin_exit();
        }),
        ("damage", |rig| {
            rig.frame_with(|status| status.health = 19);
        }),
        ("dimension", |rig| {
            rig.frame_with(|status| status.dimension = 1);
        }),
        ("handover", |rig| {
            rig.controller.on_server_instruction(ServerInstruction::SetOrClear, false);
            rig.frame();
        }),
    ];
    for (name, exit) in exits {
        let mut rig = marker_rig();
        assert!(!rig.world.is_empty(), "{name}");
        exit(&mut rig);
        assert!(rig.world.is_empty(), "{name}");
        rig.frames(EXIT_WAIT + 2);
        assert!(rig.world.is_empty(), "{name}");
    }
}

#[test]
fn world_rejoin_resends_markers() {
    let mut rig = marker_rig();
    let shown = rig.world.clone();
    rig.frame_with(|status| status.in_world = false);
    // the client drops every shape with the world, so nothing is owed for them
    rig.world.clear();
    rig.frame();
    assert!(rig.batch_sizes[rig.batch_sizes.len() - 2..].iter().all(|size| *size == 0));

    // the smooth camera is still off from the first session
    rig.start();
    rig.tap(FreecamAction::AddPoint);
    rig.walk(20);
    rig.tap(FreecamAction::AddPoint);
    rig.frames(3);

    assert_eq!(rig.world, shown);
}

#[test]
fn fov_change_over_one_percent_rebuilds() {
    let mut rig = marker_rig();
    rig.frame_with(|status| status.fov_y_degrees = 70.5);
    assert_eq!(rig.last_batch_size(), 0);
    rig.frame_with(|status| status.fov_y_degrees = 72.0);
    assert!(rig.last_batch_size() > 0);
    rig.frame();
    assert!(rig.last_batch_size() > 0);
    rig.frame_with(|status| status.aspect = 16.0 / 9.0 * 1.005);
    assert_eq!(rig.last_batch_size(), 0);
    rig.frame_with(|status| status.aspect = 16.0 / 9.0 * 1.02);
    assert!(rig.last_batch_size() > 0);
}

#[test]
fn markers_disabled_sends_nothing() {
    let config =
        FreecamConfig { markers: MarkerConfig { enabled: false, ..MarkerConfig::default() }, ..Rig::config(false) };
    let mut rig = Rig::with_config(config);
    rig.build_path(20);
    rig.frames(3);
    rig.play();
    rig.frames(EXIT_WAIT + 40);
    rig.begin_exit();
    rig.frames(EXIT_WAIT + 2);
    assert!(rig.batch_sizes.iter().all(|size| *size == 0));
}

#[test]
fn clear_points_removes_all_markers() {
    let mut rig = cursor_rig_with_points();
    rig.frames(3);
    assert!(!rig.world.is_empty());
    rig.controller.apply_ui_action(UiAction::ClearPoints);
    rig.controller.apply_ui_action(UiAction::ClearPoints);
    rig.frame();
    assert!(rig.world.is_empty());
    assert_eq!(rig.controller.state(), FreecamState::On);
}

#[test]
fn shutdown_batch_removes_every_live_marker() {
    let mut rig = marker_rig();
    let live = rig.world.len();
    let removes = rig.controller.marker_shutdown_batch().to_vec();
    assert_eq!(removes.len(), live);
    for entry in &removes {
        assert_eq!(entry.shape.kind, MarkerKind::Remove);
        rig.world.remove(&entry.id);
    }
    assert!(rig.world.is_empty());
    assert!(rig.controller.marker_batch().is_empty());
}

#[test]
fn frames_after_the_shutdown_batch_build_no_markers() {
    let mut rig = marker_rig();
    for entry in rig.controller.marker_shutdown_batch().to_vec() {
        rig.world.remove(&entry.id);
    }

    rig.tap(FreecamAction::AddPoint);
    rig.frames(3);

    assert_eq!(rig.controller.state(), FreecamState::On);
    assert!(rig.world.is_empty());
    assert!(rig.batch_sizes.iter().rev().take(3).all(|size| *size == 0));
}

#[test]
fn add_point_requests_a_ray_from_the_camera() {
    let mut rig = Rig::look_at();
    let pose = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    let out = rig.frame();
    let expected = RayRequest { origin: pose.position, direction: pose.forward(), max_distance: 256.0 };
    assert_eq!(out.ray, Some(expected));
    assert!(rig.controller.path().keys().is_empty());
    assert_eq!(rig.frame().ray, None);
}

#[test]
fn ray_hit_stores_target() {
    let mut rig = Rig::look_at();
    let pose = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    rig.frame();
    let target = Vec3::new(4.0, 60.0, 12.0);
    rig.controller.on_ray_result(Some(target));
    assert_eq!(rig.controller.path().keys(), &[PathKey::new(pose, Some(target))]);
    rig.frames(3);
    assert_eq!(rig.controller.path().keys().len(), 1);
}

#[test]
fn ray_miss_stores_plain_key() {
    let mut rig = Rig::look_at();
    let pose = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    rig.frame();
    rig.controller.on_ray_result(None);
    assert_eq!(rig.controller.path().keys(), &[PathKey::new(pose, None)]);
    rig.frames(3);
    assert_eq!(rig.controller.path().keys().len(), 1);
}

#[test]
fn no_answer_in_two_frames_stores_plain_key() {
    let mut rig = Rig::look_at();
    let pose = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    assert!(rig.frame().ray.is_some());
    rig.frame();
    assert!(rig.controller.path().keys().is_empty());
    rig.frame();
    assert_eq!(rig.controller.path().keys(), &[PathKey::new(pose, None)]);
    // an answer after the timeout changes nothing
    rig.controller.on_ray_result(Some(Vec3::new(1.0, 2.0, 3.0)));
    assert_eq!(rig.controller.path().keys(), &[PathKey::new(pose, None)]);
}

#[test]
fn look_at_disabled_adds_plain_key_at_once() {
    let mut rig = Rig::new();
    let pose = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    assert_eq!(rig.controller.path().keys(), &[PathKey::new(pose, None)]);
    assert_eq!(rig.frame().ray, None);
}

#[test]
fn play_right_after_add_point_keeps_the_new_key() {
    let mut rig = Rig::look_at();
    let first = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    rig.frame();
    rig.controller.on_ray_result(None);
    let last = rig.walk(20);
    rig.tap(FreecamAction::AddPoint);
    rig.play();
    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert_eq!(rig.controller.path().points(), &[first, last]);
}

#[test]
fn playback_waits_for_marker_removal_before_moving() {
    // 50 entries per send drains a 256-key path's markers over far more frames than the settle
    let config =
        FreecamConfig { markers: MarkerConfig { max_per_send: 50, ..MarkerConfig::default() }, ..Rig::config(false) };
    let mut rig = Rig::with_config(config);
    let first = rig.start_sharp();
    for _ in 0..256 {
        rig.tap(FreecamAction::AddPoint);
        rig.walk(2);
    }
    for _ in 0..1000 {
        rig.frame();
        if rig.last_batch_size() == 0 {
            break;
        }
    }
    assert_eq!(rig.controller.path().keys().len(), 256);
    let live = rig.world.len();
    assert!(live > 50 * EXIT_WAIT, "{live} live shapes");

    rig.play();
    let mut frames = 1;
    while Rig::pose(&rig.frame()) == first {
        frames += 1;
        assert!(frames < 1000);
    }

    assert!(rig.world.is_empty(), "{} shapes left on the first moving frame", rig.world.len());
    assert!(frames > EXIT_WAIT, "moved after {frames} frames");
}

#[test]
fn handler_recapture_resends_every_slot() {
    let mut rig = marker_rig();
    let shown = rig.world.clone();
    assert!(!shown.is_empty());
    rig.generation = 2;
    let mut resent = HashMap::new();
    for _ in 0..100 {
        rig.frame();
        let batch = rig.controller.marker_batch();
        if batch.is_empty() {
            break;
        }
        for entry in batch {
            resent.insert(entry.id, entry.shape);
        }
    }
    assert_eq!(resent, shown);
    assert_eq!(rig.world, shown);
}

#[test]
fn no_handler_sends_nothing() {
    let mut rig = Rig::new();
    rig.generation = 0;
    rig.build_path(20);
    rig.frames(3);
    assert!(rig.batch_sizes.iter().all(|size| *size == 0));
    assert!(rig.world.is_empty());

    // nothing was marked sent, so the first capture delivers every shape
    rig.generation = 1;
    rig.frames(20);
    let mut reference = marker_rig();
    reference.frames(20);
    assert!(!rig.world.is_empty());
    assert_eq!(rig.world, reference.world);
}

#[test]
fn playback_settle_is_bounded_when_markers_cannot_drain() {
    let mut rig = Rig::new();
    let (first, _) = rig.build_path(20);
    rig.frames(3);
    assert!(!rig.world.is_empty());
    rig.generation = 0;
    let defaults = MarkerConfig::default();
    let bound = EXIT_WAIT + defaults.max_shapes.div_ceil(defaults.max_per_send);

    rig.play();
    let mut frames = 1;
    while Rig::pose(&rig.frame()) == first {
        frames += 1;
        assert!(frames <= bound + 2, "still settling after {frames} frames");
    }

    assert!(frames > EXIT_WAIT, "moved after {frames} frames");
    assert_eq!(rig.controller.state(), FreecamState::Playing);
}

#[test]
fn add_moves_playhead_to_new_point_and_keeps_selection() {
    let mut rig = Rig::new();
    let [p0, p1, p2] = rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(0));
    let new = rig.walk(5);

    rig.click(UiAction::AddPoint);

    assert_eq!(rig.controller.path().points(), &[p0, new, p1, p2]);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, Some(0));
    assert_eq!(snapshot.playhead, rig.point_distance(1));

    rig.click(UiAction::SelectPoint(3));
    let early = rig.point_distance(1) * 0.5;
    rig.click(UiAction::Scrub(early));
    assert_eq!(rig.controller.ui_snapshot().selection, Some(3));
    let pose = rig.walk(3);
    rig.click(UiAction::AddPoint);

    assert_eq!(rig.controller.path().points()[1], pose);
    let after = rig.controller.ui_snapshot();
    assert_eq!(after.selection, Some(4));
    assert_eq!(rig.controller.path().points()[4], p2);
    assert_eq!(after.playhead, rig.point_distance(1));
}

#[test]
fn ctrl_b_inserts_at_the_playhead() {
    let mut rig = Rig::new();
    let [p0, p1, p2] = rig.three_points();
    rig.bar_on();
    rig.click(UiAction::Rewind);
    rig.cursor();
    let new = rig.walk(4);

    rig.tap(FreecamAction::AddPoint);

    assert_eq!(rig.controller.path().points(), &[p0, new, p1, p2]);
    assert_eq!(rig.controller.ui_snapshot().playhead, rig.point_distance(1));
}

#[test]
fn select_moves_playhead_and_camera() {
    let mut rig = Rig::new();
    let [_, p1, _] = rig.three_points();
    rig.bar_on();
    rig.walk(10);

    let out = rig.click(UiAction::SelectPoint(1));

    assert_eq!(Rig::pose(&out), p1);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, Some(1));
    assert_eq!(snapshot.playhead, rig.point_distance(1));
    assert_eq!(Rig::pose(&rig.frame()), p1);
}

#[test]
fn select_uses_look_at_aim() {
    let mut rig = Rig::look_at();
    let first = rig.start_sharp();
    rig.tap(FreecamAction::AddPoint);
    rig.frame();
    // 30 degrees off the key's own view, inside the angle where the aim is trusted
    let target = first.position + CameraPose::new(first.position, first.yaw - 30.0, first.pitch).forward() * 12.0;
    rig.controller.on_ray_result(Some(target));
    rig.walk(20);
    rig.tap(FreecamAction::AddPoint);
    rig.frame();
    rig.controller.on_ray_result(None);
    rig.bar_on();
    rig.walk(10);
    let aim = (target - first.position).normalized();

    let selected = Rig::pose(&rig.click(UiAction::SelectPoint(0)));

    assert!(Rig::gap(selected, first) < 1e-4);
    assert!(selected.forward().dot(aim) > 0.999, "forward {:?} aim {aim:?}", selected.forward());
    assert!(first.forward().dot(aim) < 0.9);

    rig.click(UiAction::SelectPoint(0));
    rig.walk(10);
    let scrubbed = Rig::pose(&rig.click(UiAction::Scrub(0.0)));
    assert!(Rig::gap(scrubbed, first) < 1e-4);
    assert!(scrubbed.forward().dot(aim) > 0.999);
}

#[test]
fn deselect_does_not_move_camera() {
    let mut rig = Rig::new();
    rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(1));
    let away = rig.walk(10);
    let playhead = rig.controller.ui_snapshot().playhead;

    let out = rig.click(UiAction::SelectPoint(1));

    assert_eq!(Rig::pose(&out), away);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, None);
    assert_eq!(snapshot.playhead, playhead);
}

#[test]
fn scrub_sets_camera_to_curve_pose() {
    let mut rig = Rig::new();
    let [_, p1, _] = rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(1));
    let length = rig.controller.path().curve().length();
    let distance = length * 0.3;
    let expected = rig.controller.path().curve().pose_at(distance);

    let out = rig.click(UiAction::Scrub(distance));

    let pose = Rig::pose(&out);
    assert!(Rig::gap(pose, expected) < 1e-4);
    assert!((pose.yaw - expected.yaw).abs() < 1e-3);
    assert!((pose.pitch - expected.pitch).abs() < 1e-3);
    assert!(Rig::gap(pose, p1) > 0.1);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.playhead, distance);
    assert_eq!(snapshot.selection, Some(1));
}

#[test]
fn replace_changes_only_selected_key() {
    let mut rig = Rig::look_at();
    let first = rig.start_sharp();
    let mut keys = vec![first];
    for _ in 0..2 {
        rig.tap(FreecamAction::AddPoint);
        rig.frame();
        rig.controller.on_ray_result(None);
        keys.push(rig.walk(20));
    }
    rig.tap(FreecamAction::AddPoint);
    rig.frame();
    rig.controller.on_ray_result(None);
    rig.bar_on();
    rig.click(UiAction::SelectPoint(1));
    let here = rig.walk(6);
    let hit = here.position + Vec3::new(3.0, 0.0, 3.0);

    let out = rig.click(UiAction::Replace);

    assert_eq!(out.ray, Some(RayRequest { origin: here.position, direction: here.forward(), max_distance: 256.0 }));
    assert_eq!(rig.controller.path().points()[1], keys[1]);
    rig.controller.on_ray_result(Some(hit));
    let stored = rig.controller.path().keys();
    assert_eq!(stored.len(), 3);
    assert_eq!(stored[0], PathKey::new(keys[0], None));
    assert_eq!(stored[1], PathKey::new(here, Some(hit)));
    assert_eq!(stored[2], PathKey::new(keys[2], None));
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, Some(1));
    assert_eq!(snapshot.playhead, rig.point_distance(1));
}

#[test]
fn replace_without_selection_does_nothing() {
    let mut rig = Rig::new();
    let points = rig.three_points();
    rig.bar_on();
    rig.walk(5);

    rig.click(UiAction::Replace);

    assert_eq!(rig.controller.path().points(), &points);
}

#[test]
fn delete_moves_playhead_back_and_clears_selection() {
    let mut rig = Rig::new();
    let [p0, p1, _] = rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(2));

    rig.click(UiAction::Delete);

    assert_eq!(rig.controller.path().points(), &[p0, p1]);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, None);
    assert_eq!(snapshot.playhead, rig.point_distance(1));

    rig.click(UiAction::SelectPoint(0));
    rig.cursor();
    rig.press_delete();
    rig.frame();

    assert_eq!(rig.controller.path().points(), &[p1]);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, None);
    assert_eq!(snapshot.playhead, 0.0);
}

#[test]
fn play_starts_at_playhead_and_from_start_at_end() {
    let mut rig = Rig::new();
    let [p0, p1, p2] = rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(1));

    let settle = rig.play();

    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert_eq!(Rig::pose(&settle), p1);
    let mut last = settle;
    for _ in 0..400 {
        if rig.controller.state() != FreecamState::Playing {
            break;
        }
        last = rig.frame();
    }
    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(Rig::pose(&last), p2);
    assert_eq!(rig.controller.ui_snapshot().playhead, rig.controller.path().curve().length());

    let again = rig.play();

    assert_eq!(rig.controller.state(), FreecamState::Playing);
    assert_eq!(Rig::pose(&again), p0);
}

#[test]
fn bar_hidden_while_playing_and_back_after() {
    let mut rig = Rig::new();
    rig.build_path(20);

    let start = rig.play();

    assert!(!start.ui_visible);
    assert!(!start.cursor_mode);
    let snapshot = rig.controller.ui_snapshot();
    assert!(snapshot.playing);
    assert!(!snapshot.visible);
    let mut back = None;
    for _ in 0..400 {
        let out = rig.frame();
        if rig.controller.state() != FreecamState::Playing {
            back = Some(out);
            break;
        }
        assert!(!out.ui_visible);
        assert!(!out.cursor_mode);
    }
    let back = back.unwrap();
    assert!(back.ui_visible);
    assert!(back.cursor_mode);
    assert!(!rig.controller.ui_snapshot().playing);
}

#[test]
fn rewind_goes_to_point_one_and_clears_selection() {
    let mut rig = Rig::new();
    let [p0, _, _] = rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(2));
    rig.walk(5);

    let out = rig.click(UiAction::Rewind);

    assert_eq!(Rig::pose(&out), p0);
    let snapshot = rig.controller.ui_snapshot();
    assert_eq!(snapshot.selection, None);
    assert_eq!(snapshot.playhead, 0.0);
}

#[test]
fn playhead_seconds_matches_playback_time() {
    let mut rig = Rig::new();
    let (first, _) = rig.build_path(80);
    let mut previous = Rig::pose(&rig.play());
    assert_eq!(previous, first);
    let mut moving = 0;
    for _ in 0..(EXIT_WAIT + 40) {
        let pose = Rig::pose(&rig.frame());
        if pose != previous {
            moving += 1;
        }
        previous = pose;
    }

    rig.stop();

    let snapshot = rig.controller.ui_snapshot();
    let elapsed = moving as f32 * FRAME_STEP as f32;
    assert!(moving > 20);
    assert!((snapshot.playhead_seconds - elapsed).abs() < 2e-3, "{} vs {elapsed}", snapshot.playhead_seconds);
    assert_eq!(snapshot.total_seconds, rig.controller.path().shot_seconds(snapshot.speed));
}

#[test]
fn bar_off_after_select_keeps_pose() {
    const TURN: i32 = 400;
    const LOOK: i32 = 20;
    let sensitivity = MotionConfig::default().sensitivity;
    let mut rig = Rig::new();
    let (first, _) = rig.build_smooth_path(20);
    // the camera turns away from the key yaw, so a look goal that was not reset would differ from it
    rig.look(TURN as f32, 0.0);
    rig.frames(40);
    rig.bar_on();
    rig.walk(10);
    let turned = Rig::pose(&rig.frames(40)[39]);
    assert!((turned.yaw - first.yaw - TURN as f32 * sensitivity).abs() < 0.5, "turned {turned:?}");

    let selected = Rig::pose(&rig.click(UiAction::SelectPoint(0)));
    let off = rig.cursor();
    let still = rig.frames(5);
    rig.look(LOOK as f32, 0.0);
    let looked = rig.frames(60);

    assert_eq!(selected, first);
    assert!(!off.cursor_mode);
    assert_eq!(Rig::pose(&off), selected);
    for out in still {
        assert_eq!(Rig::pose(&out), selected);
    }
    // the smooth camera eases from the selected yaw to the selected yaw plus the mouse turn, never past it
    let goal = selected.yaw + LOOK as f32 * sensitivity;
    let mut previous = selected.yaw;
    for out in &looked {
        let pose = Rig::pose(out);
        assert!(pose.yaw >= previous && pose.yaw <= goal + 1e-4, "yaw {} after {previous}, goal {goal}", pose.yaw);
        assert_eq!(pose.pitch, selected.pitch);
        assert_eq!(pose.position, selected.position);
        previous = pose.yaw;
    }
    assert!((previous - goal).abs() < 1e-3, "settled at {previous}, goal {goal}");
}

#[test]
fn selection_and_playhead_clamp_after_clear_and_world_change() {
    let mut rig = Rig::new();
    rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(2));

    rig.controller.apply_ui_action(UiAction::ClearPoints);
    rig.controller.apply_ui_action(UiAction::ClearPoints);

    let cleared = rig.controller.ui_snapshot();
    assert_eq!(cleared.point_count, 0);
    assert_eq!(cleared.selection, None);
    assert_eq!(cleared.playhead, 0.0);
    assert_eq!(cleared.length, 0.0);

    rig.tap(FreecamAction::AddPoint);
    rig.walk(20);
    rig.tap(FreecamAction::AddPoint);
    rig.walk(20);
    rig.tap(FreecamAction::AddPoint);
    rig.click(UiAction::SelectPoint(2));
    assert_eq!(rig.controller.ui_snapshot().selection, Some(2));

    rig.frame_with(|status| status.dimension = 1);

    let moved = rig.controller.ui_snapshot();
    assert_eq!(moved.point_count, 0);
    assert_eq!(moved.selection, None);
    assert_eq!(moved.playhead, 0.0);
    assert!(moved.point_distances.iter().all(|distance| *distance == 0.0));
}

#[test]
fn snapshot_carries_point_distances_and_speed() {
    let mut rig = Rig::new();
    rig.three_points();

    let snapshot = rig.controller.ui_snapshot();

    assert_eq!(snapshot.point_count, 3);
    for index in 0..3 {
        assert_eq!(snapshot.point_distances[index], rig.point_distance(index));
    }
    assert_eq!(snapshot.point_distances[3], 0.0);
    assert_eq!(snapshot.length, rig.controller.path().curve().length());
    assert_eq!(snapshot.speed, 6.0);
}

#[test]
fn bar_actions_need_bar_mode() {
    let mut rig = Rig::new();
    let points = rig.three_points();
    let here = rig.walk(5);

    for action in [UiAction::SelectPoint(1), UiAction::Rewind, UiAction::Scrub(1.0), UiAction::AddPoint, UiAction::Play]
    {
        assert_eq!(Rig::pose(&rig.click(action)), here);
    }

    assert_eq!(rig.controller.state(), FreecamState::On);
    assert_eq!(rig.controller.path().points(), &points);
    assert_eq!(rig.controller.ui_snapshot().selection, None);
}

#[test]
fn selecting_a_point_turns_only_its_box_green() {
    let mut rig = marker_rig();
    let first = rig.controller.path().points()[0].position;
    let last = rig.controller.path().points()[1].position;
    let green_at = |rig: &Rig, apex: Vec3| {
        rig.world.values().any(|shape| {
            shape.kind == MarkerKind::Line && shape.color == 0xFF4B_E35A && (shape.location - apex).length() < 1e-3
        })
    };
    rig.bar_on();
    rig.frames(3);
    let before = rig.batch_sizes.len();

    rig.click(UiAction::SelectPoint(0));
    rig.frames(3);

    assert!(green_at(&rig, first));
    assert!(!green_at(&rig, last));
    let sent: usize = rig.batch_sizes[before..].iter().sum();
    let box_lines = 8 * MarkerConfig::default().line_weight as usize;
    assert!(sent > 0 && sent <= 2 * box_lines, "{sent} entries sent for a selection change");

    rig.click(UiAction::SelectPoint(0));
    rig.frames(3);

    assert!(!green_at(&rig, first));
    assert!(green_at(&rig, last));
}

#[test]
fn play_from_mid_path_eases_in() {
    let mut rig = Rig::new();
    rig.three_points();
    rig.bar_on();
    rig.click(UiAction::SelectPoint(1));
    let cruise = 6.0 * FRAME_STEP as f32;

    let mut previous = Rig::pose(&rig.play());
    let mut steps = Vec::new();
    for _ in 0..400 {
        if rig.controller.state() != FreecamState::Playing || steps.len() >= 6 {
            break;
        }
        let pose = Rig::pose(&rig.frame());
        let step = Rig::gap(previous, pose);
        if step > 0.0 {
            steps.push(step);
        }
        previous = pose;
    }

    assert_eq!(steps.len(), 6);
    assert!(steps[0] < 0.1 * cruise, "first moving step {} at cruise {cruise}", steps[0]);
    assert!(steps.windows(2).all(|pair| pair[1] > pair[0]), "steps {steps:?}");
    assert!(steps.iter().all(|step| *step < cruise), "steps {steps:?}");
}

#[test]
fn path_change_rebuild_time_reaches_telemetry() {
    let mut rig = Rig::with_config(FreecamConfig { telemetry: true, ..Rig::config(false) });
    rig.build_path(4);
    rig.frames(40);
    let changed = rig.controller.last_telemetry_report().expect("first window");
    rig.frames(40);
    let quiet = rig.controller.last_telemetry_report().expect("second window");

    assert!(changed.path_rebuild_us_max > 0.0, "{changed:?}");
    assert_eq!(quiet.path_rebuild_us_max, 0.0);
}

#[test]
fn toggle_keybind_requests_start_and_granted_starts() {
    let mut rig = Rig::new();
    rig.controller
        .on_link_event(&ModEvent::Keybind { action: FreecamAction::Toggle.index(), phase: KeybindPhase::Pressed });

    assert!(rig.controller.take_start_request());
    assert!(!rig.controller.take_start_request());
    rig.controller.finish_start(StartLeases::Granted);
    let out = rig.frame();

    assert_eq!(rig.controller.state(), FreecamState::On);
    assert!(matches!(out.camera, CameraCommand::Set(_)));
    assert!(out.leases_wanted);
}

#[test]
fn denied_start_stays_off_and_sends_nothing() {
    let mut rig = Rig::new();
    rig.controller
        .on_link_event(&ModEvent::Keybind { action: FreecamAction::Toggle.index(), phase: KeybindPhase::Pressed });
    assert!(rig.controller.take_start_request());

    rig.controller.finish_start(StartLeases::Denied(Capability::Camera));
    let outs = rig.frames(3);

    assert_eq!(rig.controller.state(), FreecamState::Off);
    for out in outs {
        assert_eq!(out.camera, CameraCommand::None);
        assert!(!out.leases_wanted);
    }
}

#[test]
fn toggle_while_on_starts_the_exit_without_a_start_request() {
    let mut rig = Rig::new();
    rig.start();

    rig.controller
        .on_link_event(&ModEvent::Keybind { action: FreecamAction::Toggle.index(), phase: KeybindPhase::Pressed });

    assert!(!rig.controller.take_start_request());
    rig.frame();
    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
}

#[test]
fn fly_keybind_holds_until_released() {
    let mut rig = Rig::new();
    let start = rig.start_sharp();

    rig.hold(FreecamAction::FlyForward, true);
    let a = Rig::pose(&rig.frame());
    let b = Rig::pose(&rig.frame());
    rig.hold(FreecamAction::FlyForward, false);
    let c = Rig::pose(&rig.frame());
    let d = Rig::pose(&rig.frame());

    assert_ne!(a.position, start.position);
    assert_ne!(b.position, a.position);
    assert_eq!(c.position, d.position);
}

#[test]
fn fly_keybind_repeat_changes_nothing() {
    let mut rig = Rig::new();
    let start = rig.start_sharp();

    rig.keybind(FreecamAction::FlyForward, KeybindPhase::Repeat);
    let out = rig.frame();

    assert_eq!(Rig::pose(&out).position, start.position);
}

#[test]
fn focus_loss_clears_held_flight_keys() {
    let mut rig = Rig::new();
    rig.start_sharp();
    rig.hold(FreecamAction::FlyForward, true);
    rig.frame();

    rig.frame_with(|status| status.focused = false);
    let a = Rig::pose(&rig.frame());
    let b = Rig::pose(&rig.frame());

    assert_eq!(a.position, b.position);
}

#[test]
fn camera_suspended_in_bar_mode_exits_and_drops_cursor() {
    let mut rig = Rig::new();
    rig.start_with_cursor();
    rig.hold(FreecamAction::FlyForward, true);

    rig.event(ModEvent::Suspended { capability: Capability::Camera, by: ModIndex(1) });
    let mut exit = vec![rig.frame()];
    exit.extend(rig.frames(EXIT_WAIT - 1));

    assert!(matches!(exit[0].camera, CameraCommand::None));
    assert!(!exit[0].cursor_mode);
    assert!(exit.iter().all(|out| !matches!(out.camera, CameraCommand::Set(_))));
    let last = exit[EXIT_WAIT - 1];
    assert_eq!(last.camera, CameraCommand::Clear);
    assert!(!last.leases_wanted);
    assert_eq!(rig.controller.state(), FreecamState::Off);
}

#[test]
fn input_revoked_exits() {
    let mut rig = Rig::new();
    rig.start();

    rig.event(ModEvent::Revoked { capability: Capability::Input, by: ModIndex(2) });
    rig.frame();

    assert!(matches!(rig.controller.state(), FreecamState::Exiting { .. }));
}

#[test]
fn suspended_hud_does_not_exit() {
    let mut rig = Rig::new();
    rig.start();

    rig.event(ModEvent::Suspended { capability: Capability::Hud, by: ModIndex(1) });
    rig.frame();

    assert_eq!(rig.controller.state(), FreecamState::On);
}

#[test]
fn disabled_event_goes_off_at_once() {
    let mut rig = Rig::new();
    rig.start_with_cursor();

    rig.event(ModEvent::Disabled);
    let out = rig.frame();

    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert_eq!(out.camera, CameraCommand::Clear);
    assert!(!out.cursor_mode);
    assert!(!out.leases_wanted);
}

#[test]
fn pointer_delta_turns_the_camera_in_fly_mode_only() {
    let mut rig = Rig::new();
    let before = Rig::pose(&rig.start());

    rig.look(200.0, 0.0);
    let turned = Rig::pose(&rig.frames(30)[29]);
    rig.cursor();
    rig.look(200.0, 0.0);
    let bar = rig.frames(30);

    assert_ne!(turned.yaw, before.yaw);
    assert_eq!(Rig::pose(&bar[29]).yaw, Rig::pose(&bar[0]).yaw);
}

#[test]
fn server_instruction_from_other_mod_is_ignored() {
    let mut rig = Rig::new();
    rig.start();

    rig.controller.on_server_instruction(ServerInstruction::SetOrClear, true);
    rig.frame();

    assert_eq!(rig.controller.state(), FreecamState::On);
}

#[test]
fn apply_config_changes_speed_and_legend_live() {
    let mut rig = Rig::new();
    rig.start();
    let config = FreecamConfig {
        motion: MotionConfig { max_speed: 4.0, ..MotionConfig::default() },
        keys: KeyConfig { toggle: "Ctrl+Shift+F10".to_owned(), ..KeyConfig::default() },
        ..Rig::config(false)
    };

    rig.controller.apply_config(config);

    let exit_row = rig.controller.legend().rows.iter().find(|row| row.label == "exit freecam").unwrap();
    assert_eq!(exit_row.key, "Ctrl+Shift+F10");
    assert_eq!(rig.controller.ui_snapshot().speed, 4.0);
    assert_eq!(rig.controller.state(), FreecamState::On);
}

#[test]
fn exit_sends_no_clear_while_another_mod_holds_the_camera() {
    let mut rig = Rig::new();
    rig.start();

    rig.event(ModEvent::Suspended { capability: Capability::Camera, by: ModIndex(1) });
    let exit: Vec<FrameOutput> =
        (0..EXIT_WAIT + 2).map(|_| rig.frame_with(|s| s.camera_held_by_other = true)).collect();

    assert!(exit.iter().all(|out| out.camera == CameraCommand::None), "{exit:?}");
    assert_eq!(rig.controller.state(), FreecamState::Off);
    assert!(!exit[EXIT_WAIT + 1].leases_wanted);
}

#[test]
fn apply_config_sets_a_changed_start_speed_and_smooth_flag() {
    let mut rig = Rig::new();
    let config = FreecamConfig {
        motion: MotionConfig { start_speed: 12.0, smooth_at_start: false, ..MotionConfig::default() },
        ..Rig::config(false)
    };

    rig.controller.apply_config(config.clone());
    let start = rig.start();
    rig.hold(FreecamAction::FlyForward, true);
    let first = rig.frame();

    assert_eq!(rig.controller.ui_snapshot().speed, 12.0);
    // smooth off: the first held frame already moves at the full speed
    let moved = (Rig::pose(&first).position - Rig::pose(&start).position).length();
    assert!((moved - 12.0 * FRAME_STEP as f32).abs() < 1e-3, "{moved}");

    // the same config again leaves a wheel change alone
    rig.wheel(1.0);
    rig.frame();
    let wheeled = rig.controller.ui_snapshot().speed;
    rig.controller.apply_config(config);
    assert_eq!(rig.controller.ui_snapshot().speed, wheeled);
}
