use crate::structs::CameraPose;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum CameraCommand {
    #[default]
    None,
    Set(CameraPose),
    Clear,
}
