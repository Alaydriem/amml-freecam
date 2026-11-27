use alaydriem_freecam::services::LookRay;
use amml_api::{Aabb, ActorBox, Vec3 as ApiVec3};

fn api(x: f32, y: f32, z: f32) -> ApiVec3 {
    ApiVec3 { x, y, z }
}

fn actor(min: [f32; 3], max: [f32; 3], is_local_player: bool) -> ActorBox {
    ActorBox { aabb: Aabb { min: api(min[0], min[1], min[2]), max: api(max[0], max[1], max[2]) }, is_local_player }
}

#[test]
fn the_nearest_actor_on_the_ray_is_the_hit() {
    let actors = [actor([0.0, 0.0, 8.0], [1.0, 2.0, 9.0], false), actor([0.0, 0.0, 4.0], [1.0, 2.0, 5.0], false)];

    let hit = LookRay::nearest_actor(&actors, [0.5, 1.0, 0.0], [0.5, 1.0, 20.0]);

    let hit = hit.unwrap();
    assert!((hit[2] - (4.0 - 0.1)).abs() < 1e-4, "{hit:?}");
}

#[test]
fn the_local_player_is_never_a_target() {
    let actors = [actor([0.0, 0.0, 4.0], [1.0, 2.0, 5.0], true)];

    assert_eq!(LookRay::nearest_actor(&actors, [0.5, 1.0, 0.0], [0.5, 1.0, 20.0]), None);
}

#[test]
fn an_actor_off_the_ray_is_missed() {
    let actors = [actor([5.0, 0.0, 4.0], [6.0, 2.0, 5.0], false)];

    assert_eq!(LookRay::nearest_actor(&actors, [0.5, 1.0, 0.0], [0.5, 1.0, 20.0]), None);
}

#[test]
fn the_walk_visits_three_blocks_per_block_of_length_plus_three() {
    assert_eq!(LookRay::steps(10.0), 33);
    assert_eq!(LookRay::steps(0.5), 6);
}
