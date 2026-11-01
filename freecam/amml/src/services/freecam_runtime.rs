use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use amml_game::{ClientInstance, GameEntries, GameHeap, HeldClient, StopLatch};
use amml_sdk::{HookHandle, ModRuntime, PanicText};

use crate::services::{FrameDriver, FrameStatusReader, FreecamGame, FreecamSession, PacketCapture, RenderOverride};
use crate::types::{DeliveringScope, FrameContext, LinkState};

// what the detours share: the folders, the stop latch, the capture, the render override and the state of the
// current enable. The packet and frame detours run on the game thread; disable may run on another and waits for the
// locks
pub struct FreecamRuntime {
    config_dir: PathBuf,
    latch: StopLatch,
    client: HeldClient,
    capture: PacketCapture,
    render: RenderOverride,
    reader: FrameStatusReader,
    camera_owned: AtomicBool,
    game: Mutex<Option<Arc<FreecamGame>>>,
    session: Mutex<Option<Arc<FreecamSession>>>,
    link: Mutex<LinkState>,
    driver: Mutex<Option<FrameDriver>>,
    hooks: Mutex<Vec<HookHandle>>,
}

static RUNTIME: OnceLock<FreecamRuntime> = OnceLock::new();

impl FreecamRuntime {
    // the detours: client update, setup camera, view object, level camera, and the three packet handlers
    pub const HOOK_CAPACITY: usize = 7;

    pub fn new(config_dir: &Path) -> Self {
        Self {
            config_dir: config_dir.to_path_buf(),
            latch: StopLatch::new(),
            client: HeldClient::new(),
            capture: PacketCapture::new(),
            render: RenderOverride::new(),
            reader: FrameStatusReader::new(),
            camera_owned: AtomicBool::new(false),
            game: Mutex::new(None),
            session: Mutex::new(None),
            link: Mutex::new(LinkState::Off),
            driver: Mutex::new(None),
            hooks: Mutex::new(Vec::with_capacity(Self::HOOK_CAPACITY)),
        }
    }

    // the runtime of this DLL; the first call wins
    pub fn install(config_dir: &Path) -> &'static FreecamRuntime {
        RUNTIME.get_or_init(|| Self::new(config_dir))
    }

    pub fn get() -> Option<&'static FreecamRuntime> {
        RUNTIME.get()
    }

    // raised by disable and by a panic in a hook body
    pub fn latch(&self) -> &StopLatch {
        &self.latch
    }

    pub fn capture(&self) -> &PacketCapture {
        &self.capture
    }

    // true while freecam controls the camera, from turning on until the controller is fully off
    pub fn is_active(&self) -> bool {
        self.camera_owned.load(Ordering::Relaxed)
    }

    // enable: builds the game access from the entries and reaches the manager now; when the manager is not there
    // yet, the next tick tries again. Err names the part that could not be built
    pub fn start(&self, entries: GameEntries) -> Result<Vec<String>, String> {
        let game = FreecamGame::new(entries, GameHeap::msvc())
            .ok_or_else(|| "the camera write or the delivery layout check refused the entries".to_string())?;
        let lines = game.refused_lines();
        *Self::lock(&self.game) = Some(Arc::new(game));
        *Self::lock(&self.link) = LinkState::Waiting;
        self.latch.lower();
        self.attach(false);
        Ok(lines)
    }

    // keeps an installed hook; false, keeping nothing, when HOOK_CAPACITY hooks are already kept
    pub fn keep_hook(&self, hook: HookHandle) -> bool {
        let mut hooks = Self::lock(&self.hooks);
        if hooks.len() >= Self::HOOK_CAPACITY {
            return false;
        }
        hooks.push(hook);
        true
    }

    // the installed hooks, newest first, leaving none behind
    pub fn take_hooks(&self) -> Vec<HookHandle> {
        let mut hooks = std::mem::take(&mut *Self::lock(&self.hooks));
        hooks.reverse();
        hooks
    }

    // the body of the client update detour
    pub fn tick(&self, client: NonNull<c_void>) {
        self.guarded("the tick", || {
            self.client.set(client.as_ptr());
            if *Self::lock(&self.link) == LinkState::Waiting {
                self.attach(true);
            }
            self.with_frame(|context, driver, client| driver.tick(context, client));
        });
    }

    // before setupCamera: the frame of a world
    pub fn before_setup_camera(&self) {
        self.guarded("the frame", || {
            self.with_frame(|context, driver, client| driver.before_setup_camera(context, client));
        });
    }

    // after setupCamera: the free camera's rotation, then the projection read
    #[allow(unsafe_code)]
    pub fn after_setup_camera(&self, camera: usize) {
        self.guarded("the camera hook", || {
            let Some(game) = self.game() else { return };
            // SAFETY: `camera` is the Camera argument of the setupCamera call that just returned, live for it
            unsafe {
                self.render.apply_rotation(&game.writer, camera as *mut c_void);
                self.reader.observe_camera(&game.entries, camera as *const c_void);
            }
        });
    }

    // after createViewRenderObject: the free camera's position in the view object the game just filled
    #[allow(unsafe_code)]
    pub fn after_view_object(&self, view_object: usize) {
        self.guarded("the view hook", || {
            let Some(game) = self.game() else { return };
            // SAFETY: `view_object` is the ViewRenderObject buffer createViewRenderObject just filled
            unsafe { self.render.apply_view(&game.writer, view_object as *mut c_void) };
        });
    }

    // after LevelRendererCamera::preRenderUpdate: the free camera's position for the chunk sort
    #[allow(unsafe_code)]
    pub fn after_pre_render_update(&self, level_camera: usize) {
        self.guarded("the level camera hook", || {
            let Some(game) = self.game() else { return };
            // SAFETY: `level_camera` is the `this` of the preRenderUpdate call that just returned
            unsafe { self.render.apply_level(&game.writer, level_camera as *mut c_void) };
        });
    }

    // a packet the server always sends reached the client handler: the capture
    pub fn on_handle(&self, handler: usize, id: usize) {
        self.guarded("the capture hook", || self.capture.capture(handler as *mut c_void, id as *const c_void));
    }

    // a camera instruction reached the client handler; the server's are reported to core, the mod's own are not
    #[allow(unsafe_code)]
    pub fn on_camera_instruction(&self, packet: usize) {
        if DeliveringScope::active() {
            return;
        }
        self.guarded("the camera instruction hook", || {
            let Some(game) = self.game() else { return };
            let session = self.session();
            // SAFETY: `packet` is the CameraInstructionPacket the handler is called with, live for the call
            unsafe {
                game.instructions.report(packet as *const c_void, |kind| {
                    game.watch.on_server_instruction(kind);
                    if let Some(session) = &session {
                        let _ = session.server_instruction(kind.server());
                    }
                });
            }
        });
    }

    // disable, before the hooks go: the camera returns to the player and the markers are removed. Nothing is sent
    // once the stop latch is raised
    pub fn send_shutdown(&self) -> (bool, usize) {
        let mut sent = (false, 0);
        if self.latch.raised() || ModRuntime::is_disabled() {
            curia::info!("shutdown: the stop latch is raised; nothing is sent");
            return sent;
        }
        self.guarded("the shutdown", || {
            let camera_owned = self.is_active();
            let (Some(game), Some(session)) = (self.game(), self.session()) else { return };
            let mut driver = Self::lock(&self.driver);
            let Some(driver) = driver.as_mut() else { return };
            let context = self.context(&game, &session);
            let client = self.client.get(&self.latch);
            // SAFETY: the held client is the live ClientInstance of the last tick
            #[allow(unsafe_code)]
            let client = client.and_then(|client| unsafe { ClientInstance::from_raw(client.as_ptr(), &game.entries) });
            sent = driver.shutdown(context, client.as_ref(), camera_owned, &self.latch);
        });
        sent
    }

    // disable, after the hooks are gone: the capture and the packets are dropped and the registration ends
    pub fn stop(&self) {
        self.latch.raise();
        if let Some(game) = Self::lock(&self.game).take() {
            game.reset();
        }
        self.capture.reset();
        self.render.reset();
        self.reader.reset();
        self.client.clear();
        self.camera_owned.store(false, Ordering::Relaxed);
        *Self::lock(&self.driver) = None;
        *Self::lock(&self.session) = None;
        *Self::lock(&self.link) = LinkState::Off;
    }

    pub fn is_attached(&self) -> bool {
        *Self::lock(&self.link) == LinkState::Attached
    }

    // runs one frame with the held client, when the game, the session and the driver are there
    #[allow(unsafe_code)]
    fn with_frame(
        &self,
        body: impl FnOnce(
            FrameContext<'_>,
            &mut FrameDriver,
            &ClientInstance<'_>,
        ) -> Option<freecam_core::structs::FrameOutput>,
    ) {
        let (Some(game), Some(session)) = (self.game(), self.session()) else { return };
        let Some(client) = self.client.get(&self.latch) else { return };
        // SAFETY: the held client is the `this` of the running ClientInstance::update or of the frame after it
        let Some(client) = (unsafe { ClientInstance::from_raw(client.as_ptr(), &game.entries) }) else { return };
        let mut driver = Self::lock(&self.driver);
        let Some(driver) = driver.as_mut() else { return };
        if let Some(output) = body(self.context(&game, &session), driver, &client) {
            self.camera_owned.store(output.camera_owned, Ordering::Relaxed);
        }
    }

    fn context<'a>(&'a self, game: &'a FreecamGame, session: &'a FreecamSession) -> FrameContext<'a> {
        FrameContext {
            game,
            session,
            capture: &self.capture,
            render: &self.render,
            reader: &self.reader,
            camera_owned: self.is_active(),
        }
    }

    fn game(&self) -> Option<Arc<FreecamGame>> {
        Self::lock(&self.game).clone()
    }

    fn session(&self) -> Option<Arc<FreecamSession>> {
        Self::lock(&self.session).clone()
    }

    fn guarded(&self, what: &str, body: impl FnOnce()) {
        if self.latch.raised() || ModRuntime::is_disabled() {
            return;
        }
        if let Err(payload) = catch_unwind(AssertUnwindSafe(body)) {
            self.latch.raise();
            if let Some(runtime) = ModRuntime::get() {
                runtime.logger().error(&format!("{what} panicked: {}; freecam stops", PanicText::of(payload.as_ref())));
            }
        }
    }

    fn attach(&self, last_try: bool) {
        match ApiTable::resolve().and_then(|table| FreecamSession::create(table, &self.config_dir)) {
            Ok(session) => {
                *Self::lock(&self.driver) = Some(FrameDriver::new(session.marker_capacity()));
                *Self::lock(&self.session) = Some(Arc::new(session));
                *Self::lock(&self.link) = LinkState::Attached;
            }
            Err(error) if last_try => {
                Self::log_link_error(&error);
                *Self::lock(&self.link) = LinkState::Off;
            }
            Err(_) => {}
        }
    }

    fn log_link_error(error: &LinkError) {
        let Some(runtime) = ModRuntime::get() else { return };
        match error {
            LinkError::ManagerMissing | LinkError::AbiMismatch { .. } => {
                runtime.logger().error("@alaydriem/freecam needs @amml/modmanager; freecam stays off");
            }
            other => {
                runtime.logger().error(&format!("@alaydriem/freecam could not start ({other}); freecam stays off"));
            }
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
