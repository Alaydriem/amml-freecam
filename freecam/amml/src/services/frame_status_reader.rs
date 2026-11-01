use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use amml_game::{ClientInstance, GameEntries, Projection};
use freecam_core::structs::{CameraPose, FrameStatus, Vec3};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::services::{CameraInstructions, CameraWatch, PacketCapture};

// reads the game state core needs once per frame. The field of view and the aspect come from the projection matrix
// after setupCamera, so they are one frame old, as in the LeviLamina build
pub struct FrameStatusReader {
    fov_bits: AtomicU32,
    aspect_bits: AtomicU32,
    logged_generation: AtomicU32,
}

impl FrameStatusReader {
    pub const fn new() -> Self {
        Self { fov_bits: AtomicU32::new(0), aspect_bits: AtomicU32::new(0), logged_generation: AtomicU32::new(0) }
    }

    // seconds since the first call; a steady clock
    pub fn now_seconds() -> f64 {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_secs_f64()
    }

    // whether the foreground window belongs to this process
    #[allow(unsafe_code)]
    pub fn game_window_focused() -> bool {
        // SAFETY: GetForegroundWindow takes no arguments; a null result means no window has focus
        let window = unsafe { GetForegroundWindow() };
        if window.is_null() {
            return false;
        }
        let mut process = 0u32;
        // SAFETY: `window` is a window handle from the system and `process` a writable u32
        unsafe { GetWindowThreadProcessId(window, &mut process) };
        // SAFETY: GetCurrentProcessId takes no arguments and cannot fail
        process == unsafe { GetCurrentProcessId() }
    }

    /// after setupCamera: keeps fov y and aspect of the camera's projection; 0 for both when the matrix is degenerate
    /// or the projection entries are off, and core then uses the screen the mod manager draws on
    ///
    /// # Safety
    /// `camera` must be the live Camera of the setupCamera call that just returned
    #[allow(unsafe_code)]
    pub unsafe fn observe_camera(&self, entries: &GameEntries, camera: *const c_void) {
        // SAFETY: forwarded to the caller
        let (fov, aspect) = unsafe { Projection::read(entries, camera) }.unwrap_or((0.0, 0.0));
        self.fov_bits.store(fov.to_bits(), Ordering::Relaxed);
        self.aspect_bits.store(aspect.to_bits(), Ordering::Relaxed);
    }

    pub fn projection(&self) -> (f32, f32) {
        (
            f32::from_bits(self.fov_bits.load(Ordering::Relaxed)),
            f32::from_bits(self.aspect_bits.load(Ordering::Relaxed)),
        )
    }

    pub fn reset(&self) {
        self.fov_bits.store(0, Ordering::Relaxed);
        self.aspect_bits.store(0, Ordering::Relaxed);
        self.logged_generation.store(0, Ordering::Relaxed);
    }

    // the status outside a world: the time and the focus only
    pub fn outside_world(now: f64, focused: bool) -> FrameStatus {
        FrameStatus {
            time_seconds: now,
            in_world: false,
            menu_open: false,
            focused,
            ready: false,
            player_camera_active: false,
            start_pose: CameraPose::default(),
            health: 0,
            dimension: 0,
            rendered_position: None,
            game_camera_forward: None,
            game_camera_position: None,
            fov_y_degrees: 0.0,
            aspect: 0.0,
            last_shape_send_us: 0.0,
            handler_generation: 0,
            present_count: 0,
            camera_held_by_other: false,
        }
    }

    // the status in a world: the player's head, look angles, health and dimension, whether the free preset is ready,
    // and which camera the game shows. Game thread
    #[allow(unsafe_code)]
    pub fn read(
        &self,
        client: &ClientInstance<'_>,
        instructions: &CameraInstructions,
        capture: &PacketCapture,
        watch: &CameraWatch,
        rendered: Option<Vec3>,
        camera_owned: bool,
    ) -> FrameStatus {
        let mut status = Self::outside_world(Self::now_seconds(), Self::game_window_focused());
        status.rendered_position = rendered;
        (status.fov_y_degrees, status.aspect) = self.projection();
        status.menu_open = client.is_showing_menu();
        if let Some(player) = client.local_player() {
            status.in_world = client.is_world_active();
            let level = player.level().map_or(std::ptr::null_mut(), |level| level.as_ptr());
            // SAFETY: the level of the live local player
            status.ready = unsafe { instructions.ready(capture, level) };
            if let (Some(head), Some(pose)) = (player.head_position(), player.pose()) {
                status.start_pose = CameraPose::new(Vec3::new(head[0], head[1], head[2]), pose.yaw, pose.pitch);
            }
            status.health = player.health().unwrap_or(0);
            status.dimension = player.dimension().and_then(|dimension| dimension.id()).unwrap_or(0);
        }
        // SAFETY: the live ClientInstance of this frame
        status.player_camera_active = unsafe {
            watch.player_camera_active(client.as_ptr(), capture.generation(), status.time_seconds, camera_owned)
        };
        self.log_first_world_frame(capture.generation(), &status);
        status
    }

    // one line for the first frame of each captured handler (each world) that has a player, so a run shows what the
    // reads gave
    fn log_first_world_frame(&self, generation: u32, status: &FrameStatus) {
        if generation == 0
            || !status.in_world
            || self.logged_generation.swap(generation, Ordering::Relaxed) == generation
        {
            return;
        }
        curia::info!("first world frame", {
            generation: generation,
            ready: status.ready,
            player_camera_active: status.player_camera_active,
            health: status.health,
            dimension: status.dimension,
            fov_y_degrees: status.fov_y_degrees,
            aspect: status.aspect,
        });
    }
}

impl Default for FrameStatusReader {
    fn default() -> Self {
        Self::new()
    }
}
