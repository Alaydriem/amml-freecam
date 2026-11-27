use alaydriem_freecam::FreecamMod;
use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::amml_abi::{
    AmmlMouseAction, AmmlMouseButton, AmmlPhase, CameraInstructionReceived, CameraSetup, ClientTick, KeyInput,
    MouseInput, WindowFocus, WorldLeft,
};
use amml_api::testing::{FakeCamera, FakeClient, FakeGame, FakeWorld};
use amml_api::{Area, BlockHit, CameraKind, Projection, Rotation, Vec3};

// freecam enabled on a fake game that serves every row of its `USES`, connected to the fake manager, in a world with
// the local player at 10, 64, 10 and the client packet handler seen
pub struct FreecamRig {
    pub game: FakeGame,
    pub client: FakeClient,
    pub camera: FakeCamera,
}

impl FreecamRig {
    pub const KEY_F3: u16 = 0x72;
    pub const KEY_P: u16 = 0x50;
    pub const KEY_W: u16 = 0x57;
    pub const KEY_B: u16 = 0x42;
    pub const KEY_CTRL: u16 = 0xA2;

    pub fn new() -> Self {
        Self::build(true)
    }

    // as `new`, but the loader has not seen the client packet handler yet
    pub fn without_handler() -> Self {
        Self::build(false)
    }

    fn build(handler_seen: bool) -> Self {
        let game = FakeTable::game();
        game.offer::<ClientTick>();
        game.offer::<CameraSetup>();
        game.offer::<CameraInstructionReceived>();
        game.offer::<KeyInput>();
        game.offer::<MouseInput>();
        game.offer::<WorldLeft>();
        game.offer::<WindowFocus>();
        for area in [
            Area::CLIENT_READ,
            Area::ACTOR_READ.at(3),
            Area::ACTOR_HEALTH,
            Area::CAMERA_POSE,
            Area::CAMERA_PROJECTION,
            Area::CAMERA_FREE,
            Area::WORLD_SHAPES,
            Area::CAMERA_ACTIVE,
            Area::WORLD_RAYCAST,
            Area::WORLD_ACTORS_IN,
            Area::OVERLAY_SCREENSHOT,
        ] {
            game.offer_area(area);
        }
        FakeTable::provide(&game.ctx()).unwrap();
        FakeTable::set_config("");
        game.load::<FreecamMod>().unwrap();
        game.enable::<FreecamMod>().unwrap();
        game.open_gate();
        game.set_epoch(1);
        if handler_seen {
            game.capture_handler();
            game.set_target_epoch(1);
        }
        let client = FakeClient {
            local_player: true,
            world_active: true,
            position: Some(Vec3 { x: 10.0, y: 64.0, z: 10.0 }),
            head_position: Some(Vec3 { x: 10.0, y: 65.6, z: 10.0 }),
            rotation: Some(Rotation { yaw: 0.0, pitch: 0.0 }),
            dimension: Some(0),
            health: Some(20),
            active_camera: Some(CameraKind::FirstPerson),
            world: Some(FakeWorld::default()),
            ..FakeClient::default()
        };
        game.set_client(client.clone());
        let camera =
            FakeCamera { projection: Some(Projection { fov_y: 70.0, aspect: 16.0 / 9.0 }), ..FakeCamera::default() };
        Self { game, client, camera }
    }

    // one rendered world frame on the game thread: the camera setup Before and After, with the fake client and camera
    pub fn frame(&self) {
        self.game.raise_camera_setup(AmmlPhase::BEFORE, &self.camera);
        self.game.raise_camera_setup(AmmlPhase::AFTER, &self.camera);
    }

    // a second camera setup of the frame that ran last
    pub fn second_setup(&self) {
        let mut event = CameraSetup {
            header: FakeGame::header::<CameraSetup>(),
            frame_id: 0,
            first_in_frame: 0,
            _reserved: [0; 7],
        };
        self.game.raise_with_client(&mut event);
    }

    // a client update in the world (the loader raises it every frame besides the camera setup)
    pub fn tick(&self) {
        self.game.raise_with_client(&mut ClientTick { header: FakeGame::header::<ClientTick>() });
    }

    pub fn frames(&self, count: usize) {
        for _ in 0..count {
            self.frame();
        }
    }

    // the manager's toggle keybind (freecam does not hold its lease yet), then one frame
    pub fn toggle_on(&self) {
        FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed });
        self.frame();
        FakeTable::push_event(ModEvent::Keybind { action: 0, phase: KeybindPhase::Released });
        self.frame();
    }

    // F3+P from the keyboard, as freecam matches it while it holds its lease, then enough frames for the exit wait
    pub fn toggle_off(&self) {
        self.game.real_key(Self::KEY_F3, true);
        self.game.real_key(Self::KEY_P, true);
        self.game.real_key(Self::KEY_P, false);
        self.game.real_key(Self::KEY_F3, false);
        self.frames(30);
    }

    // Ctrl+B from the keyboard, then a few frames for the look-at ray
    pub fn add_point(&self) {
        self.game.real_key(Self::KEY_CTRL, true);
        self.game.real_key(Self::KEY_B, true);
        self.game.real_key(Self::KEY_B, false);
        self.game.real_key(Self::KEY_CTRL, false);
        self.frames(3);
    }

    pub fn set_client(&mut self, change: impl FnOnce(&mut FakeClient)) {
        change(&mut self.client);
        self.game.set_client(self.client.clone());
    }

    pub fn set_ray_hit(&mut self, hit: Option<BlockHit>) {
        self.set_client(|client| client.world = Some(FakeWorld { ray_hit: hit, ..FakeWorld::default() }));
    }

    pub fn focus(&self, gained: bool) {
        let mut event = WindowFocus {
            header: FakeGame::header::<WindowFocus>(),
            gained: u8::from(gained),
            replayed: 0,
            _reserved: [0; 2],
        };
        self.game.raise(&mut event);
    }

    // a real mouse move of the raw counts `dx`, `dy`
    pub fn mouse_move(&self, dx: i32, dy: i32) {
        let mut event = MouseInput { action: AmmlMouseAction::MOVE, dx, dy, ..Self::mouse() };
        self.game.raise(&mut event);
    }

    pub fn mouse_wheel(&self, notches: f32) {
        let mut event = MouseInput { action: AmmlMouseAction::WHEEL, notches, ..Self::mouse() };
        self.game.raise(&mut event);
    }

    fn mouse() -> MouseInput {
        MouseInput {
            header: FakeGame::header::<MouseInput>(),
            action: AmmlMouseAction::MOVE,
            button: AmmlMouseButton::LEFT,
            down: 0,
            _reserved: 0,
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            notches: 0.0,
        }
    }

    pub fn world_left(&self) {
        self.game.raise(&mut WorldLeft { header: FakeGame::header::<WorldLeft>(), epoch: 1 });
    }

    // the free camera calls that went out, as the fake lists them
    pub fn free_calls(&self) -> Vec<String> {
        self.game.camera_free()
    }

    pub fn sets(&self) -> usize {
        self.free_calls().iter().filter(|call| call.starts_with("set_free")).count()
    }

    pub fn logged(&self, start: &str) -> bool {
        self.game.logs().iter().any(|line| line.starts_with(start))
    }

    pub fn lines(&self, start: &str) -> Vec<String> {
        self.game.logs().into_iter().filter(|line| line.starts_with(start)).collect()
    }
}
