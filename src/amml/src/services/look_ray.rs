use amml_api::{Aabb, ActorBox, BlockSource, RayOptions};
use freecam_core::structs::Vec3;

use crate::services::PoseConvert;
use crate::types::RayBox;

// the look-at ray: the first block along the view (every block a candidate, against its selection outline), then
// the nearest actor box in front of that block. Game thread, from the world frame
pub struct LookRay;

impl LookRay {
    // a voxel walk crosses at most one block face per axis for each block of length, plus the start and end blocks
    const STEPS_PER_BLOCK: u32 = 3;
    const EXTRA_STEPS: u32 = 3;

    // the hit point of the ray from `origin` along `direction` for at most `max_distance` blocks; None for a miss and
    // for a ray that is not finite (no game call is made then)
    pub fn cast(source: &BlockSource<'_>, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<Vec3> {
        if !origin.is_finite() || !direction.is_finite() || !max_distance.is_finite() || max_distance <= 0.0 {
            return None;
        }
        let to = origin + direction * max_distance;
        let options = RayOptions { every_block: true, outline: true, max_steps: Self::steps(max_distance) };
        let block = source.raycast(PoseConvert::vec_to_api(origin), PoseConvert::vec_to_api(to), options);
        let end = block.map_or(to, |hit| PoseConvert::vec_from_api(hit.point));
        let from = [origin.x, origin.y, origin.z];
        let end = [end.x, end.y, end.z];
        let search = RayBox::spanning(from, end);
        let actors = source.actors_in(Aabb {
            min: amml_api::Vec3 { x: search.min[0], y: search.min[1], z: search.min[2] },
            max: amml_api::Vec3 { x: search.max[0], y: search.max[1], z: search.max[2] },
        });
        let actor = Self::nearest_actor(&actors, from, end).map(|[x, y, z]| Vec3::new(x, y, z));
        actor.or(block.map(|hit| PoseConvert::vec_from_api(hit.point)))
    }

    // the voxels the walk may visit for a ray of `max_distance` blocks
    pub fn steps(max_distance: f32) -> u32 {
        let blocks = max_distance.ceil().clamp(0.0, u32::MAX as f32) as u32;
        blocks.saturating_mul(Self::STEPS_PER_BLOCK).saturating_add(Self::EXTRA_STEPS)
    }

    // the nearest hit on an actor box (grown by the default pick margin) between `from` and `end`; the local player
    // is never a target
    pub fn nearest_actor(actors: &[ActorBox], from: [f32; 3], end: [f32; 3]) -> Option<[f32; 3]> {
        let mut nearest: Option<([f32; 3], f32)> = None;
        for actor in actors.iter().filter(|actor| !actor.is_local_player) {
            let (min, max) = (actor.aabb.min, actor.aabb.max);
            let bounds = RayBox::from_bounds([min.x, min.y, min.z, max.x, max.y, max.z]);
            let Some(hit) = bounds.grown(RayBox::DEFAULT_MARGIN).hit(from, end) else { continue };
            let distance = (0..3).map(|axis| (hit[axis] - from[axis]).powi(2)).sum::<f32>();
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((hit, distance));
            }
        }
        nearest.map(|(hit, _)| hit)
    }
}
