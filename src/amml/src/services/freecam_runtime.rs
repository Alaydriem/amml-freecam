use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use amml_api::amml_abi::{AmmlOrigin, CameraSetup, KeyInput, MouseInput};
use amml_api::{CameraSetupExt, Ctx, EventCtx, InstructionKind, Logger};

use crate::services::{FrameDriver, FrameStatusReader, InputBridge};

// what the listeners share for the life of the DLL: the frame driver of the current enable (None while disabled),
// the input path, the projection of the last camera setup and the window focus. The frame listeners run on the game
// thread; the key, mouse and focus listeners may run on other threads and touch only the input path and the focus
pub struct FreecamRuntime {
    driver: Mutex<Option<FrameDriver>>,
    input: InputBridge,
    reader: FrameStatusReader,
    focused: AtomicBool,
    // a focus loss since the last frame; the next frame clears the held flight keys
    focus_lost: AtomicBool,
    // camera instructions since the last frame; the loader raises the echo of a free camera call from inside that
    // call, while the frame holds the driver, so instructions wait here for the next frame
    instructions: Mutex<Vec<(InstructionKind, AmmlOrigin)>>,
    // instructions dropped at the bound since the last frame
    instructions_dropped: AtomicU32,
}

static RUNTIME: OnceLock<FreecamRuntime> = OnceLock::new();

impl FreecamRuntime {
    // instructions kept between two frames; more in one frame are dropped (a server sends a few at most)
    pub const INSTRUCTION_CAPACITY: usize = 16;

    pub fn new() -> Self {
        Self {
            driver: Mutex::new(None),
            input: InputBridge::new(),
            reader: FrameStatusReader::new(),
            focused: AtomicBool::new(true),
            focus_lost: AtomicBool::new(false),
            instructions: Mutex::new(Vec::with_capacity(Self::INSTRUCTION_CAPACITY)),
            instructions_dropped: AtomicU32::new(0),
        }
    }

    // the runtime of this DLL; the first call makes it
    pub fn get() -> &'static FreecamRuntime {
        RUNTIME.get_or_init(Self::new)
    }

    // enable: the frames run with this driver until `stop`
    pub fn start(&self, driver: FrameDriver) {
        self.focused.store(true, Ordering::Relaxed);
        self.focus_lost.store(false, Ordering::Relaxed);
        *self.lock() = Some(driver);
    }

    // disable, after the listeners are gone: the driver goes, which closes the manager link. The loader releases the
    // free camera and removes the shapes itself
    pub fn stop(&self, log: &Logger) {
        let driver = self.lock().take();
        drop(driver);
        self.input.set_gate(false, false, log);
    }

    // `ClientTick`: the frame outside a world
    pub fn tick(&self, at: &EventCtx) {
        let mut driver = self.lock();
        if let Some(driver) = driver.as_mut() {
            self.pass_instructions(driver);
            driver.outside_frame(at, &self.reader, &self.input, &self.focus_lost);
        }
    }

    // `CameraSetup` Before: the frame in a world
    pub fn before_camera_setup(&self, event: &CameraSetup, at: &EventCtx) {
        let mut driver = self.lock();
        if let Some(driver) = driver.as_mut() {
            self.pass_instructions(driver);
            driver.world_frame(event, at, &self.reader, &self.input, &self.focus_lost);
        }
    }

    // `CameraSetup` After: the projection the game set up, for the next frame's marker sizes
    pub fn after_camera_setup(&self, event: &CameraSetup) {
        if let Some(camera) = event.camera() {
            self.reader.observe(camera.projection());
        }
    }

    // `CameraInstructionReceived`: this mod's own echo is dropped here; the others wait for the next frame
    pub fn camera_instruction(&self, kind: InstructionKind, origin: AmmlOrigin) {
        if origin == AmmlOrigin::THIS_MOD {
            return;
        }
        let mut pending = self.instructions.lock().unwrap_or_else(PoisonError::into_inner);
        if pending.len() < Self::INSTRUCTION_CAPACITY {
            pending.push((kind, origin));
        } else {
            self.instructions_dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn world_left(&self) {
        self.instructions.lock().unwrap_or_else(PoisonError::into_inner).clear();
        if let Some(driver) = self.lock().as_mut() {
            driver.world_left();
        }
    }

    pub fn key(&self, event: &KeyInput, ctx: &Ctx) {
        self.input.on_key(event, ctx.input().modifiers(), &ctx.log());
    }

    pub fn mouse(&self, event: &MouseInput) {
        self.input.on_mouse(event);
    }

    // a window focus message. Every loss that is not a replay asks the next frame to clear the held flight keys, also
    // when the focus was lost already; a gain only changes the tracked state. A change of the state is logged
    pub fn focus(&self, gained: bool, replayed: bool, log: &Logger) {
        if replayed {
            return;
        }
        if !gained {
            self.focus_lost.store(true, Ordering::Relaxed);
        }
        if self.focused.swap(gained, Ordering::Relaxed) != gained {
            log.debug("window focus", &[("focused", &gained.to_string())]);
        }
    }

    fn pass_instructions(&self, driver: &mut FrameDriver) {
        let pending = std::mem::take(&mut *self.instructions.lock().unwrap_or_else(PoisonError::into_inner));
        for (kind, origin) in pending {
            driver.camera_instruction(kind, origin);
        }
        let dropped = self.instructions_dropped.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            driver.instructions_dropped(dropped);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<FrameDriver>> {
        self.driver.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for FreecamRuntime {
    fn default() -> Self {
        Self::new()
    }
}
