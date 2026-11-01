// The game functions freecam hooks. The render detours write the camera (HookOrder::WRITE); the packet detours only
// capture or report (HookOrder::READ_AFTER_WRITERS)
pub mod camera_instruction;
pub mod camera_presets;
pub mod chunk_publisher;
pub mod client_update;
pub mod handle_fn;
pub mod pre_render_update;
pub mod setup_camera;
pub mod view_render_object;
