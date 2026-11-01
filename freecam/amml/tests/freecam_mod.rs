use alaydriem_freecam::FreecamMod;
use alaydriem_freecam::types::{FreecamEntry, FreecamFeature};
use std::collections::BTreeSet;

use amml_game::{EntryId, GameError, Projection};

use crate::support::{FakeGame, FakeResolver};

#[test]
fn with_every_entry_no_feature_is_off() {
    let entries = FreecamMod::resolve(&FakeResolver::new(&FakeGame::values())).unwrap();

    assert!(entries.off_features().is_empty(), "{:?}", entries.off_features());
    assert!(FreecamMod::off_lines(&entries).is_empty());
    for feature in FreecamFeature::OPTIONAL {
        assert!(FreecamFeature::is_on(&entries, feature), "{feature}");
    }
}

#[test]
fn every_entry_is_asked_for_once() {
    let wanted = FreecamMod::wanted();
    let ids: Vec<EntryId> = wanted.iter().map(|(id, _)| *id).collect();

    for (index, id) in ids.iter().enumerate() {
        assert!(!ids[index + 1..].contains(id), "{} twice", id.as_str());
    }
}

#[test]
fn a_missing_look_at_entry_turns_only_look_at_off_with_one_line() {
    let values: Vec<_> =
        FakeGame::values().into_iter().filter(|(id, _)| *id != FreecamEntry::BLOCK_SOURCE_CLIP).collect();

    let entries = FreecamMod::resolve(&FakeResolver::new(&values)).unwrap();
    let lines = FreecamMod::off_lines(&entries);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("look-at is off"), "{lines:?}");
    assert!(FreecamFeature::is_on(&entries, FreecamFeature::MARKERS));
}

#[test]
fn the_projection_read_needs_only_the_camera_anchor_the_write_requires() {
    let values: Vec<_> = FakeGame::values().into_iter().filter(|(id, _)| *id != EntryId::CAMERA_FORWARD).collect();

    let result = FreecamMod::resolve(&FakeResolver::new(&values));

    assert_eq!(Projection::ENTRIES, [EntryId::CAMERA_FORWARD]);
    assert!(matches!(result, Err(GameError::MissingEntry { id }) if id == "Camera.mForward"));
}

#[test]
fn amml_hcl_lists_every_entry_the_mod_asks_for() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/amml.hcl")).unwrap();
    let manifest: hcl::Value = hcl::from_str(&text).unwrap();
    let entries = manifest.as_object().and_then(|object| object.get("entries")).and_then(hcl::Value::as_array).unwrap();
    let listed: BTreeSet<String> = entries.iter().map(|entry| entry.as_str().unwrap().to_string()).collect();
    let wanted: BTreeSet<String> = FreecamMod::wanted().iter().map(|(id, _)| id.as_str().to_string()).collect();

    assert_eq!(listed, wanted);
}

#[test]
fn a_missing_delivery_entry_refuses_the_enable() {
    let values: Vec<_> =
        FakeGame::values().into_iter().filter(|(id, _)| *id != FreecamEntry::INSTRUCTION_SPLINE).collect();

    let result = FreecamMod::resolve(&FakeResolver::new(&values));

    assert!(matches!(result, Err(GameError::MissingEntry { id }) if id == "CameraInstruction.mSpline"));
}

#[test]
fn the_mod_id_is_namespaced() {
    assert_eq!(<FreecamMod as amml_sdk::AmmlMod>::id(), "@alaydriem/freecam");
}
