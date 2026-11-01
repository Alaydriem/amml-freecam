use std::ffi::c_void;

// one node of the pool map of the game's entt registry (20h bytes): the index of the next node in the bucket chain,
// the component type id (the key) and the shared_ptr to the storage (the value)
#[repr(C)]
pub struct PoolNodeLayout {
    pub next: usize,         // 0x00 next node index, usize::MAX at the end
    pub type_id: u32,        // 0x08 key: entt type hash
    _pad: u32,               // 0x0C
    pub pool: *const c_void, // 0x10 shared_ptr<basic_sparse_set> element
    _control: *const c_void, // 0x18 shared_ptr control block
}

const _: () = assert!(size_of::<PoolNodeLayout>() == 0x20);
