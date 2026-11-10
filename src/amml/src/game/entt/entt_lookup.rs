use std::ffi::c_void;

use crate::game::entt::{EnttRegistryLayout, EnttStorageLayout};

// the component lookup of the game's entt, as its own inline lookups do it: the pool of a type id in the registry's
// pool map, then the entity's packed index in the pool's sparse pages, then the component in the payload pages. The
// entity traits and page sizes are compile-time constants of the game's entt
pub struct EnttLookup;

impl EnttLookup {
    // the entity index bits of an EntityId; the rest is the version
    pub const INDEX_MASK: u32 = 0x3_FFFF;
    // sparse pages of 2048 entries
    pub const SPARSE_PAGE_SHIFT: u32 = 11;
    // payload pages of 128 components
    pub const PAYLOAD_PAGE_SHIFT: u32 = 7;

    /// the storage of the component with entt type id `type_id` in `registry`; None when the registry has none
    ///
    /// # Safety
    /// `registry` must be null or a live entt registry of the game, read on the thread that owns it
    #[allow(unsafe_code)]
    pub unsafe fn storage(registry: *const c_void, type_id: u32) -> Option<*const EnttStorageLayout> {
        if registry.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live registry, laid out as the mirror
        let registry = unsafe { &*registry.cast::<EnttRegistryLayout>() };
        let buckets = registry.buckets.as_slice();
        let nodes = registry.nodes.as_slice();
        if buckets.is_empty() {
            return None;
        }
        let mut next = buckets[(buckets.len() - 1) & type_id as usize];
        // a chain is never longer than the node vector; the bound keeps a broken chain from looping
        for _ in 0..=nodes.len() {
            let node = nodes.get(next)?;
            if node.type_id == type_id {
                return (!node.pool.is_null()).then_some(node.pool.cast());
            }
            next = node.next;
        }
        None
    }

    /// the packed index of `entity` in `storage`; None when the storage does not hold it
    ///
    /// # Safety
    /// `storage` must be a live entt storage of the game
    #[allow(unsafe_code)]
    pub unsafe fn packed_index(storage: *const EnttStorageLayout, entity: u32) -> Option<u32> {
        let index = entity & Self::INDEX_MASK;
        // SAFETY: the caller guarantees a live storage, laid out as the mirror
        let pages = unsafe { (*storage).sparse.as_slice() };
        let page = *pages.get((index >> Self::SPARSE_PAGE_SHIFT) as usize)?;
        if page.is_null() {
            return None;
        }
        let slot = (index & ((1 << Self::SPARSE_PAGE_SHIFT) - 1)) as usize;
        // SAFETY: a sparse page holds 1 << SPARSE_PAGE_SHIFT entries
        let value = unsafe { page.add(slot).read() };
        ((value ^ (entity & !Self::INDEX_MASK)) < Self::INDEX_MASK).then_some(value & Self::INDEX_MASK)
    }

    /// the component of `entity` in `storage`, whose components are `T`; None when the storage does not hold the
    /// entity
    ///
    /// # Safety
    /// `storage` must be a live entt storage of the game whose components are `T`
    #[allow(unsafe_code)]
    pub unsafe fn component<T>(storage: *const EnttStorageLayout, entity: u32) -> Option<*const T> {
        // SAFETY: forwarded to the caller
        let packed = unsafe { Self::packed_index(storage, entity) }?;
        // SAFETY: as above
        let pages = unsafe { (*storage).payload.as_slice() };
        let page = *pages.get((packed >> Self::PAYLOAD_PAGE_SHIFT) as usize)?;
        if page.is_null() {
            return None;
        }
        let slot = (packed & ((1 << Self::PAYLOAD_PAGE_SHIFT) - 1)) as usize;
        Some(page.cast::<T>().wrapping_add(slot))
    }
}
