use std::ffi::c_void;

use freecam_core::structs::Vec3;

use crate::game::LookLayout;
use crate::services::{PacketCapture, ThreadGate};
use crate::types::RayBox;

// the look-at ray: the first block along the view through BlockSource::clip, then the nearest actor box in front of
// that block. The level is read only on the thread that handles the server's packets
pub struct LookRay {
    layout: LookLayout,
    gate: ThreadGate,
}

impl LookRay {
    // a voxel walk crosses at most one block face per axis for each block of length, plus the start and end blocks
    const STEPS_PER_BLOCK: i32 = 3;
    const EXTRA_STEPS: i32 = 3;

    pub fn new(layout: LookLayout) -> Self {
        Self { layout, gate: ThreadGate::new("look ray") }
    }

    // the refusal lines the thread gate logged
    pub fn refusals(&self) -> u32 {
        self.gate.warnings()
    }

    /// the hit point of the ray from `origin` along `direction` for at most `max_distance` blocks; None for a miss,
    /// for a ray that is not finite, and on any thread but the capture thread (no game call is made then)
    ///
    /// # Safety
    /// `player` must be the live local player and `region` its live BlockSource, on the game thread
    #[allow(unsafe_code)]
    pub unsafe fn cast(
        &self,
        capture: &PacketCapture,
        player: *mut c_void,
        region: *mut c_void,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<Vec3> {
        if !origin.is_finite() || !direction.is_finite() || !max_distance.is_finite() || max_distance <= 0.0 {
            return None;
        }
        if !self.gate.allows(capture.thread()) || region.is_null() {
            return None;
        }
        let from = [origin.x, origin.y, origin.z];
        let to = origin + direction * max_distance;
        let to = [to.x, to.y, to.z];
        let steps =
            (max_distance.ceil() as i32).saturating_mul(Self::STEPS_PER_BLOCK).saturating_add(Self::EXTRA_STEPS);
        // SAFETY: forwarded to the caller
        let block = unsafe { self.layout.clip(region, from, to, steps) };
        let end = block.unwrap_or(to);
        // SAFETY: forwarded to the caller
        let actor = unsafe { self.nearest_actor(player, region, from, end) };
        actor.or(block).map(|[x, y, z]| Vec3::new(x, y, z))
    }

    // the nearest hit on an actor box (grown by the default pick margin) between from and end, with the local player
    // never a target. With no source actor the search box spans from..end, so it follows the ray and stops at the
    // block hit
    #[allow(unsafe_code)]
    unsafe fn nearest_actor(
        &self,
        player: *mut c_void,
        region: *mut c_void,
        from: [f32; 3],
        end: [f32; 3],
    ) -> Option<[f32; 3]> {
        let search = RayBox::spanning(from, end);
        // SAFETY: forwarded to the caller of `cast`
        let actors = unsafe { self.layout.actors(region, search.bounds()) };
        let mut nearest: Option<([f32; 3], f32)> = None;
        for &actor in actors {
            if actor.is_null() || actor == player {
                continue;
            }
            // SAFETY: the game listed the actor as live in the region
            let Some(bounds) = (unsafe { self.layout.actor_box(actor) }) else { continue };
            let Some(hit) = RayBox::from_bounds(bounds).grown(RayBox::DEFAULT_MARGIN).hit(from, end) else { continue };
            let distance = (0..3).map(|axis| (hit[axis] - from[axis]).powi(2)).sum::<f32>();
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((hit, distance));
            }
        }
        nearest.map(|(hit, _)| hit)
    }
}
