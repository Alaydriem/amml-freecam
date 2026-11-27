use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use amml_api::{Client, Projection, Vec3 as ApiVec3};
use freecam_core::structs::{CameraPose, FrameStatus};

use crate::services::PoseConvert;

// reads the game state core needs once per frame. The field of view and the aspect come from the camera's projection
// after the camera setup, so they are one frame old
pub struct FrameStatusReader {
    start: Instant,
    fov_bits: AtomicU32,
    aspect_bits: AtomicU32,
}

impl FrameStatusReader {
    pub fn new() -> Self {
        Self { start: Instant::now(), fov_bits: AtomicU32::new(0), aspect_bits: AtomicU32::new(0) }
    }

    // seconds since the reader was made; a steady clock
    pub fn now_seconds(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    // after the camera setup: keeps fov y and aspect; 0 for both without a projection, and core then uses the screen
    // the mod manager draws on
    pub fn observe(&self, projection: Option<Projection>) {
        let projection = projection.unwrap_or_default();
        self.fov_bits.store(projection.fov_y.to_bits(), Ordering::Relaxed);
        self.aspect_bits.store(projection.aspect.to_bits(), Ordering::Relaxed);
    }

    pub fn projection(&self) -> (f32, f32) {
        (
            f32::from_bits(self.fov_bits.load(Ordering::Relaxed)),
            f32::from_bits(self.aspect_bits.load(Ordering::Relaxed)),
        )
    }

    // the status outside a world: the time and the focus only
    pub fn outside_world(&self, focused: bool) -> FrameStatus {
        FrameStatus {
            time_seconds: self.now_seconds(),
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

    // the status in a world: the player's head, look angles, health and dimension, whether the free preset is there,
    // which camera the game shows, and where the loader drew the camera last frame. Game thread
    pub fn read(
        &self,
        client: &Client<'_>,
        focused: bool,
        free_available: bool,
        player_camera_active: bool,
        rendered: Option<ApiVec3>,
    ) -> FrameStatus {
        let mut status = self.outside_world(focused);
        status.rendered_position = rendered.map(PoseConvert::vec_from_api);
        (status.fov_y_degrees, status.aspect) = self.projection();
        status.menu_open = client.showing_menu().unwrap_or(false);
        status.player_camera_active = player_camera_active;
        if let Some(player) = client.local_player() {
            status.in_world = client.world_active().unwrap_or(false);
            status.ready = free_available;
            if let (Some(head), Some(rotation)) = (player.head_position(), player.rotation()) {
                status.start_pose = PoseConvert::start_pose(head, rotation);
            }
            status.health = player.health().unwrap_or(0);
            status.dimension = player.dimension().and_then(|dimension| dimension.id()).unwrap_or(0);
        }
        status
    }
}

impl Default for FrameStatusReader {
    fn default() -> Self {
        Self::new()
    }
}
