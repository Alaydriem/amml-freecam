use freecam_core::services::PathStore;
use freecam_core::services::path_store::PATH_CAPACITY;
use freecam_core::structs::math::Vec3;
use freecam_core::structs::{CameraPose, PathKey};

struct Fixture;

impl Fixture {
    fn key(x: f32) -> PathKey {
        PathKey::new(CameraPose::new(Vec3::new(x, 0.0, 0.0), 0.0, 0.0), None)
    }
}

#[test]
fn remove_at_drops_that_point_and_bumps_revision() {
    let mut store = PathStore::new(PATH_CAPACITY);
    store.insert_at(store.len(), Fixture::key(1.0));
    store.insert_at(store.len(), Fixture::key(2.0));
    store.insert_at(store.len(), Fixture::key(3.0));
    let before = store.revision();
    assert!(store.remove_at(1));
    assert_eq!(store.keys(), &[Fixture::key(1.0), Fixture::key(3.0)]);
    assert!(store.revision() > before);
}

#[test]
fn remove_at_past_the_end_changes_nothing() {
    let mut store = PathStore::new(PATH_CAPACITY);
    store.insert_at(store.len(), Fixture::key(1.0));
    let before = store.revision();
    assert!(!store.remove_at(1));
    assert_eq!(store.len(), 1);
    assert_eq!(store.revision(), before);
}

#[test]
fn clear_empties() {
    let mut store = PathStore::new(PATH_CAPACITY);
    store.insert_at(store.len(), Fixture::key(1.0));
    let before = store.revision();
    store.clear();
    assert_eq!(store.len(), 0);
    assert!(store.keys().is_empty());
    assert!(store.revision() > before);
}

#[test]
fn insert_at_puts_the_key_at_the_index_and_bumps_revision() {
    let mut store = PathStore::new(PATH_CAPACITY);
    store.insert_at(store.len(), Fixture::key(1.0));
    store.insert_at(store.len(), Fixture::key(3.0));
    let before = store.revision();

    assert!(store.insert_at(1, Fixture::key(2.0)));
    assert!(store.insert_at(0, Fixture::key(0.0)));

    assert_eq!(store.keys(), &[Fixture::key(0.0), Fixture::key(1.0), Fixture::key(2.0), Fixture::key(3.0)]);
    assert!(store.revision() > before);
}

#[test]
fn insert_at_past_the_end_appends() {
    let mut store = PathStore::new(PATH_CAPACITY);
    assert!(store.insert_at(5, Fixture::key(1.0)));
    assert!(store.insert_at(99, Fixture::key(2.0)));
    assert_eq!(store.keys(), &[Fixture::key(1.0), Fixture::key(2.0)]);
}

#[test]
fn insert_at_capacity_returns_false() {
    let mut store = PathStore::new(PATH_CAPACITY);
    for i in 0..PATH_CAPACITY {
        assert!(store.insert_at(store.len(), Fixture::key(i as f32)));
    }
    let before = store.revision();

    assert!(!store.insert_at(0, Fixture::key(999.0)));

    assert_eq!(store.len(), PATH_CAPACITY);
    assert_eq!(store.keys()[0], Fixture::key(0.0));
    assert_eq!(store.revision(), before);
}

#[test]
fn replace_at_changes_only_that_key() {
    let mut store = PathStore::new(PATH_CAPACITY);
    store.insert_at(store.len(), Fixture::key(1.0));
    store.insert_at(store.len(), Fixture::key(2.0));
    store.insert_at(store.len(), Fixture::key(3.0));
    let before = store.revision();

    assert!(store.replace_at(1, Fixture::key(9.0)));

    assert_eq!(store.keys(), &[Fixture::key(1.0), Fixture::key(9.0), Fixture::key(3.0)]);
    assert!(store.revision() > before);
}

#[test]
fn replace_at_past_the_end_changes_nothing() {
    let mut store = PathStore::new(PATH_CAPACITY);
    store.insert_at(store.len(), Fixture::key(1.0));
    let before = store.revision();

    assert!(!store.replace_at(1, Fixture::key(9.0)));

    assert_eq!(store.keys(), &[Fixture::key(1.0)]);
    assert_eq!(store.revision(), before);
}
