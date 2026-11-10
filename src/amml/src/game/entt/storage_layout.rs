use std::mem::offset_of;

use amml_msvc::MsvcVec;

// the head of the game's entt storage of one component, up to its payload pages: the sparse set (its vtable, the
// sparse page vector, the packed vector and its bookkeeping), then the payload page vector
#[repr(C)]
pub struct EnttStorageLayout {
    _vtable: usize,                  // 0x00
    pub sparse: MsvcVec<*const u32>, // 0x08 sparse pages
    _packed: [u64; 6],               // 0x20 packed, info, mode, head
    pub payload: MsvcVec<*const u8>, // 0x50 payload pages
}

const _: () = assert!(offset_of!(EnttStorageLayout, sparse) == 0x08);
const _: () = assert!(offset_of!(EnttStorageLayout, payload) == 0x50);
