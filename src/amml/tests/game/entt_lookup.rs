use std::ffi::c_void;

use alaydriem_freecam::game::entt::{EnttLookup, EnttStorageLayout};

const TYPE_ID: u32 = 0x4F60_47C7;
const ENTITY: u32 = (3 << 18) | 5;

// a registry with one pool (TYPE_ID) in a map of 4 buckets, whose storage holds ENTITY at packed index 2, with u64
// components; every buffer is leaked and 8-byte aligned
struct FakeEntt {
    registry: *const c_void,
    storage: *const EnttStorageLayout,
}

impl FakeEntt {
    fn new() -> Self {
        let components: &mut [u64] = Box::leak(vec![10, 11, 12, 13].into_boxed_slice());
        let payload_pages: &mut [usize] = Box::leak(vec![components.as_ptr() as usize].into_boxed_slice());
        let mut sparse = vec![u32::MAX; 2048];
        sparse[5] = (3 << 18) | 2;
        let sparse: &mut [u32] = Box::leak(sparse.into_boxed_slice());
        let sparse_pages: &mut [usize] = Box::leak(vec![sparse.as_ptr() as usize].into_boxed_slice());
        let mut storage = vec![0usize; 0x68 / 8];
        storage[1] = sparse_pages.as_ptr() as usize;
        storage[2] = storage[1] + 8;
        storage[10] = payload_pages.as_ptr() as usize;
        storage[11] = storage[10] + 8;
        let storage: &mut [usize] = Box::leak(storage.into_boxed_slice());
        // node 0: another type; node 1: TYPE_ID, chained from node 0
        let nodes: &mut [usize] = Box::leak(
            vec![1, 0x1234, 0, 0, usize::MAX, TYPE_ID as usize, storage.as_ptr() as usize, 0].into_boxed_slice(),
        );
        let buckets: &mut [usize] = Box::leak(vec![usize::MAX; 4].into_boxed_slice());
        buckets[TYPE_ID as usize & 3] = 0;
        let mut registry = vec![0usize; 0x80 / 8];
        registry[9] = buckets.as_ptr() as usize;
        registry[10] = registry[9] + 4 * 8;
        registry[13] = nodes.as_ptr() as usize;
        registry[14] = registry[13] + 2 * 32;
        let registry: &mut [usize] = Box::leak(registry.into_boxed_slice());
        Self { registry: registry.as_ptr().cast(), storage: storage.as_ptr().cast() }
    }
}

#[test]
#[allow(unsafe_code)]
fn the_pool_of_a_type_is_found_through_its_bucket_chain() {
    let entt = FakeEntt::new();

    // SAFETY: a fake registry laid out as the mirror
    let (found, missing) =
        unsafe { (EnttLookup::storage(entt.registry, TYPE_ID), EnttLookup::storage(entt.registry, TYPE_ID ^ 4)) };

    assert_eq!(found, Some(entt.storage));
    assert_eq!(missing, None);
}

#[test]
#[allow(unsafe_code)]
fn the_component_of_an_entity_is_read_through_the_pages() {
    let entt = FakeEntt::new();

    // SAFETY: a fake storage laid out as the mirror, with u64 components
    let component = unsafe { EnttLookup::component::<u64>(entt.storage, ENTITY) };

    // SAFETY: the component pointer points into the fake page
    assert_eq!(component.map(|at| unsafe { at.read() }), Some(12));
}

#[test]
#[allow(unsafe_code)]
fn another_version_or_an_absent_entity_is_not_held() {
    let entt = FakeEntt::new();

    // SAFETY: as above
    let (old_version, absent) = unsafe {
        (EnttLookup::packed_index(entt.storage, (2 << 18) | 5), EnttLookup::packed_index(entt.storage, (3 << 18) | 6))
    };

    assert_eq!((old_version, absent), (None, None));
}
