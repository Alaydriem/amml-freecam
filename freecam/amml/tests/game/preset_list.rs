use alaydriem_freecam::game::PresetList;

use crate::support::FakeGame;

#[test]
fn the_free_preset_is_found_by_its_index() {
    let presets = PresetList::new(&FakeGame::entries()).unwrap();
    let level = FakeGame::level(&["minecraft:first", "minecraft:free"]);

    // SAFETY: a fake level with the fake preset list
    let index = unsafe { presets.index_of(level, PresetList::FREE_PRESET) };

    assert_eq!(index, Some(1));
}

#[test]
fn a_list_without_the_free_preset_gives_none() {
    let presets = PresetList::new(&FakeGame::entries()).unwrap();
    let level = FakeGame::level(&["custom:orbit"]);

    // SAFETY: as above
    assert_eq!(unsafe { presets.index_of(level, PresetList::FREE_PRESET) }, None);
    // SAFETY: a null level is allowed
    assert_eq!(unsafe { presets.index_of(std::ptr::null_mut(), PresetList::FREE_PRESET) }, None);
}
