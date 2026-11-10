use std::mem::offset_of;

use amml_msvc::MsvcVec;

use crate::game::entt::PoolNodeLayout;

// the head of the game's entt::basic_registry<EntityId>, up to the pool map (entt dense_map<id_type,
// shared_ptr<basic_sparse_set>>): the bucket vector of node indexes (a bucket is type id & (count - 1)), the hasher,
// then the node vector. The context before it is padding
#[repr(C)]
pub struct EnttRegistryLayout {
    _context: [u64; 9],                 // 0x00 vars
    pub buckets: MsvcVec<usize>,        // 0x48 pools: sparse (bucket heads)
    _hasher: usize,                     // 0x60
    pub nodes: MsvcVec<PoolNodeLayout>, // 0x68 pools: packed (nodes)
}

const _: () = assert!(offset_of!(EnttRegistryLayout, buckets) == 0x48);
const _: () = assert!(offset_of!(EnttRegistryLayout, nodes) == 0x68);
