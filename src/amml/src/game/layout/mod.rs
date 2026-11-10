// `#[repr(C)]` mirrors of the game structs freecam reads and writes, laid out as the 26.51 headers; entries check
// the sizes where the game code holds them
pub mod aabb_shape_layout;
pub mod camera_component_layout;
pub mod camera_instruction_layout;
pub mod camera_preset_layout;
pub mod camera_presets_layout;
pub mod entity_context_layout;
pub mod hit_result_layout;
pub mod set_instruction_layout;

pub use aabb_shape_layout::AabbShapeLayout;
pub use camera_component_layout::CameraComponentLayout;
pub use camera_instruction_layout::CameraInstructionLayout;
pub use camera_preset_layout::CameraPresetLayout;
pub use camera_presets_layout::CameraPresetsLayout;
pub use entity_context_layout::EntityContextLayout;
pub use hit_result_layout::HitResultLayout;
pub use set_instruction_layout::SetInstructionLayout;
