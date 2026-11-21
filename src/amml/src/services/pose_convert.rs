use amml_api::{Pose, Rotation, Vec3 as ApiVec3};
use freecam_core::structs::{CameraPose, Vec3};

// the one place where the loader's poses and core's poses meet. Both use the same angles (yaw 0 looks along +Z, 90
// along -X; positive pitch looks down), so the numbers pass through unchanged
pub struct PoseConvert;

impl PoseConvert {
    // core's camera pose as the loader's pose; core has no roll
    pub fn to_api(pose: CameraPose) -> Pose {
        Pose {
            position: Self::vec_to_api(pose.position),
            rotation: Rotation { yaw: pose.yaw, pitch: pose.pitch },
            roll: 0.0,
        }
    }

    // the player's head and rotation as core's start pose
    pub fn start_pose(head: ApiVec3, rotation: Rotation) -> CameraPose {
        CameraPose::new(Self::vec_from_api(head), rotation.yaw, rotation.pitch)
    }

    pub fn vec_to_api(value: Vec3) -> ApiVec3 {
        ApiVec3 { x: value.x, y: value.y, z: value.z }
    }

    pub fn vec_from_api(value: ApiVec3) -> Vec3 {
        Vec3::new(value.x, value.y, value.z)
    }
}
