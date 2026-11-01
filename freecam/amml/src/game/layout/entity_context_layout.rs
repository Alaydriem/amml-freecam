use std::ffi::c_void;
use std::mem::offset_of;

// the game's EntityContext (18h bytes): the EntityRegistry, the entt registry and the entity id. CameraRegistry holds
// them as std::optional (20h bytes, checked by `CameraRegistry.cameraEntityStride`)
#[repr(C)]
pub struct EntityContextLayout {
    _registry: *mut c_void,  // 0x00 mRegistry
    pub entt: *const c_void, // 0x08 mEnTTRegistry
    pub entity: u32,         // 0x10 mEntity
}

const _: () = assert!(offset_of!(EntityContextLayout, entity) == 0x10);
const _: () = assert!(size_of::<EntityContextLayout>() == 0x18);
