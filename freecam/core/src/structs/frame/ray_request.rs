use crate::structs::Vec3;

// a ray the mod layer casts against the world and answers through the controller's ray result
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RayRequest {
    pub origin: Vec3,
    // unit vector
    pub direction: Vec3,
    // blocks
    pub max_distance: f32,
}
