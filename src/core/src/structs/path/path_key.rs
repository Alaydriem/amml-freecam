use crate::structs::CameraPose;
use crate::structs::math::Vec3;

// one recorded point of the path
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PathKey {
    pub pose: CameraPose,
    // the point the camera aims at for a look-at key, None for a plain key
    pub target: Option<Vec3>,
}

impl PathKey {
    pub fn new(pose: CameraPose, target: Option<Vec3>) -> Self {
        Self { pose, target }
    }
}
