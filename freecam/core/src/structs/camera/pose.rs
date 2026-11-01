use crate::structs::math::Vec3;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CameraPose {
    pub position: Vec3,
    // degrees, positive turns right
    pub yaw: f32,
    // degrees, positive looks down
    pub pitch: f32,
}

impl CameraPose {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self { position, yaw, pitch }
    }

    // unit vector the camera looks along
    pub fn forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.to_radians().sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.to_radians().sin_cos();
        Vec3::new(-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch)
    }
}
