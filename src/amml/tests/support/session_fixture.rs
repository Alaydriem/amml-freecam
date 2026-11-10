use std::collections::HashMap;
use std::path::Path;

use alaydriem_freecam::services::FreecamSession;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use freecam_core::structs::{CameraPose, FrameOutput, FrameStatus, FreecamAction, MarkerEntry, MarkerKind, Vec3};

// a live session registered with this thread's fake mod manager
pub struct SessionFixture {
    pub session: FreecamSession,
}

impl SessionFixture {
    pub const FRAME_STEP: f64 = 0.05;
    pub const NO_LOOK_AT: &str = "look_at { enabled = false }";

    pub fn new() -> Self {
        Self::with_config("")
    }

    // a live session whose config.hcl, as the fake manager hands it over, is `hcl`
    pub fn with_config(hcl: &str) -> Self {
        FakeTable::set_config(hcl);
        let session = FreecamSession::create(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR)).unwrap();
        Self { session }
    }

    // queues the manager's keybind event for the next frame
    pub fn keybind(action: FreecamAction, phase: KeybindPhase) {
        FakeTable::push_event(ModEvent::Keybind { action: action.index(), phase });
    }

    // presses and releases the keybind
    pub fn tap(action: FreecamAction) {
        Self::keybind(action, KeybindPhase::Pressed);
        Self::keybind(action, KeybindPhase::Released);
    }

    pub fn start_pose() -> CameraPose {
        CameraPose::new(Vec3::new(10.0, 64.0, -5.0), 90.0, 10.0)
    }

    // in world, focused, ready, handler captured once, player camera active, health 20
    pub fn ready_status() -> FrameStatus {
        FrameStatus {
            time_seconds: 0.05,
            in_world: true,
            menu_open: false,
            focused: true,
            ready: true,
            player_camera_active: true,
            start_pose: Self::start_pose(),
            health: 20,
            dimension: 0,
            rendered_position: None,
            game_camera_forward: None,
            game_camera_position: None,
            fov_y_degrees: 70.0,
            aspect: 16.0 / 9.0,
            last_shape_send_us: 0.0,
            handler_generation: 1,
            present_count: 0,
            camera_held_by_other: false,
        }
    }

    pub fn session(&self) -> &FreecamSession {
        &self.session
    }

    // the frame's output and its marker batch
    pub fn frame(&self, status: &FrameStatus) -> (Vec<MarkerEntry>, FrameOutput) {
        let mut markers = Vec::with_capacity(self.session.marker_capacity());
        let output = self.session.frame(status, &mut markers).unwrap();
        (markers, output)
    }

    // the ready status at `time_seconds` with this handler capture count
    pub fn status_at(time_seconds: f64, handler_generation: u32) -> FrameStatus {
        FrameStatus { time_seconds, handler_generation, ..Self::ready_status() }
    }

    // applies a batch the way the client does: a remove drops the id, any other kind draws it
    pub fn apply(world: &mut HashMap<u64, MarkerEntry>, batch: &[MarkerEntry]) {
        for entry in batch {
            if entry.shape.kind == MarkerKind::Remove {
                world.remove(&entry.id);
            } else {
                world.insert(entry.id, *entry);
            }
        }
    }

    // turns freecam on and records two points 20 walking frames apart, every frame with this handler capture count;
    // returns the time of the last frame and the shapes the client shows
    pub fn build_path(&self, handler_generation: u32) -> (f64, HashMap<u64, MarkerEntry>) {
        let mut world = HashMap::new();
        let mut time = Self::FRAME_STEP;
        let step = |time: f64, world: &mut HashMap<u64, MarkerEntry>| {
            let (batch, _) = self.frame(&Self::status_at(time, handler_generation));
            Self::apply(world, &batch);
        };
        Self::tap(FreecamAction::Toggle);
        step(time, &mut world);
        Self::tap(FreecamAction::AddPoint);
        Self::keybind(FreecamAction::FlyForward, KeybindPhase::Pressed);
        for _ in 0..20 {
            time += Self::FRAME_STEP;
            step(time, &mut world);
        }
        Self::keybind(FreecamAction::FlyForward, KeybindPhase::Released);
        Self::tap(FreecamAction::AddPoint);
        for _ in 0..3 {
            time += Self::FRAME_STEP;
            step(time, &mut world);
        }
        (time, world)
    }
}
