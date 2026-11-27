use alaydriem_freecam::services::PoseConvert;
use amml_api::{Pose, Rotation, Vec3 as ApiVec3};
use freecam_core::structs::{CameraPose, Vec3};

#[test]
fn a_core_pose_becomes_the_loader_pose_with_no_roll() {
    let pose = PoseConvert::to_api(CameraPose::new(Vec3::new(1.0, 2.0, 3.0), 90.0, -30.0));

    assert_eq!(
        pose,
        Pose {
            position: ApiVec3 { x: 1.0, y: 2.0, z: 3.0 },
            rotation: Rotation { yaw: 90.0, pitch: -30.0 },
            roll: 0.0
        }
    );
}

#[test]
fn the_player_head_and_rotation_become_the_start_pose() {
    let start = PoseConvert::start_pose(ApiVec3 { x: 4.0, y: 70.5, z: -2.0 }, Rotation { yaw: 180.0, pitch: 10.0 });

    assert_eq!(start, CameraPose::new(Vec3::new(4.0, 70.5, -2.0), 180.0, 10.0));
}

#[test]
fn the_loader_and_core_angles_look_the_same_way() {
    // the loader's rule: yaw 90 looks along -X; core's forward for yaw 90 must agree
    let forward = CameraPose::new(Vec3::default(), 90.0, 0.0).forward();

    assert!((forward.x + 1.0).abs() < 1e-5 && forward.z.abs() < 1e-5, "{forward:?}");
}
