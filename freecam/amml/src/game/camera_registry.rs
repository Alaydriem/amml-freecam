use std::ffi::c_void;

use amml_game::GameEntries;
use amml_game::game::RawObject;
use amml_msvc::{MsvcVec, Optional, SharedPtr};

use crate::game::entt::EnttLookup;
use crate::game::layout::{CameraComponentLayout, EntityContextLayout};
use crate::types::FreecamEntry;

// the cameras the game knows: ClientInstance::getCameraRegistry, the camera entities of the registry and their
// CameraComponent and ActiveCameraComponent in the entt storages
#[derive(Debug, Clone, Copy)]
pub struct CameraRegistry {
    get_registry: u64,
    entities: usize,
    camera_type: u32,
    active_type: u32,
}

// `Bedrock::NonOwnerPointer<CameraRegistry>`: the shared_ptr to the control block (whose first byte is the valid
// flag), then the pointer
#[repr(C)]
struct NonOwnerPointer {
    control: SharedPtr<u8>,
    pointer: *mut c_void,
}

impl CameraRegistry {
    // the cameras the player sees the world through
    pub const PLAYER_CAMERAS: [&'static str; 3] =
        ["minecraft:first_person", "minecraft:third_person", "minecraft:third_person_front"];

    // None when an entry is missing or a checked size does not match its mirror: the camera entity stride (an
    // optional EntityContext) and the CameraComponent size (built from a factor and a shift)
    pub fn new(entries: &GameEntries) -> Option<Self> {
        let factor = entries.get(FreecamEntry::CAMERA_COMPONENT_SIZE_FACTOR)?;
        let shift = u32::try_from(entries.get(FreecamEntry::CAMERA_COMPONENT_SIZE_SHIFT)?).ok()?;
        let fits = factor.checked_shl(shift)? == size_of::<CameraComponentLayout>() as u64
            && entries.get(FreecamEntry::CAMERA_ENTITY_STRIDE)? == size_of::<Optional<EntityContextLayout>>() as u64;
        if !fits {
            return None;
        }
        Some(Self {
            get_registry: entries.get(FreecamEntry::GET_CAMERA_REGISTRY)?,
            entities: usize::try_from(entries.get(FreecamEntry::CAMERA_ENTITIES)?).ok()?,
            camera_type: u32::try_from(entries.get(FreecamEntry::CAMERA_COMPONENT_TYPE_ID)?).ok()?,
            active_type: u32::try_from(entries.get(FreecamEntry::ACTIVE_CAMERA_TYPE_ID)?).ok()?,
        })
    }

    /// whether a player camera is the active one; None when the registry is not there. Game thread
    ///
    /// # Safety
    /// `client` must be null or the live ClientInstance
    #[allow(unsafe_code)]
    pub unsafe fn player_camera_active(&self, client: *mut c_void) -> Option<bool> {
        if client.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live ClientInstance whose vtable holds getCameraRegistry at the slot
        let function = unsafe { RawObject::slot(client, self.get_registry) }?;
        // SAFETY: a non-static member that returns the NonOwnerPointer through the buffer after `this`
        let call: unsafe extern "C" fn(*mut c_void, *mut NonOwnerPointer) -> *mut NonOwnerPointer =
            unsafe { std::mem::transmute(function) };
        let mut out = std::mem::MaybeUninit::<NonOwnerPointer>::zeroed();
        // SAFETY: `out` is a buffer for the 24-byte NonOwnerPointer
        unsafe { call(client, out.as_mut_ptr()) };
        // SAFETY: the game constructed the pointer in `out`
        let mut pointer = unsafe { out.assume_init() };
        let valid = pointer.control.get().is_some_and(|flag| *flag == 1);
        let active = (valid && !pointer.pointer.is_null()).then(|| {
            // SAFETY: a valid NonOwnerPointer points to the live CameraRegistry
            unsafe { self.walk(pointer.pointer) }
        });
        // SAFETY: the copy the game handed out holds one use of the control block, given back here
        unsafe { pointer.control.release() };
        active
    }

    #[allow(unsafe_code)]
    unsafe fn walk(&self, registry: *mut c_void) -> bool {
        // SAFETY: the registry holds the vector of camera entities at the entry offset
        let cameras =
            unsafe { &*registry.cast::<u8>().add(self.entities).cast::<MsvcVec<Optional<EntityContextLayout>>>() };
        cameras.as_slice().iter().filter_map(Optional::get).any(|context| {
            // SAFETY: the entt registry of a live, engaged context
            let Some(storage) = (unsafe { EnttLookup::storage(context.entt, self.camera_type) }) else { return false };
            // SAFETY: the CameraComponent storage
            let Some(camera) = (unsafe { EnttLookup::component::<CameraComponentLayout>(storage, context.entity) })
            else {
                return false;
            };
            // SAFETY: a live CameraComponent, laid out as the mirror
            let name = unsafe { (*camera).id.text.as_bytes() };
            // SAFETY: as above
            let active = unsafe { EnttLookup::storage(context.entt, self.active_type) }
                // SAFETY: the ActiveCameraComponent storage
                .is_some_and(|storage| unsafe { EnttLookup::packed_index(storage, context.entity) }.is_some());
            active && Self::PLAYER_CAMERAS.iter().any(|player| player.as_bytes() == name)
        })
    }
}
