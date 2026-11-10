use alaydriem_freecam::services::FreecamDescriptor;
use amm_core::services::descriptor::DescriptorChecker;
use amm_core::structs::descriptor::{Capability, ClaimMode, Descriptor, KeyScope};
use freecam_core::config::FreecamConfig;
use freecam_core::structs::FreecamAction;

struct Fixture;

impl Fixture {
    fn descriptor() -> Descriptor {
        DescriptorChecker::new(128, 256).parse(FreecamDescriptor::DESCRIPTOR).unwrap()
    }

    // puts `value` at the dotted `path` inside `root`, making the objects on the way
    fn insert(root: &mut hcl::Map<String, hcl::Value>, path: &str, value: hcl::Value) {
        match path.split_once('.') {
            None => {
                root.insert(path.to_owned(), value);
            }
            Some((head, rest)) => {
                let entry = root.entry(head.to_owned()).or_insert_with(|| hcl::Value::Object(hcl::Map::new()));
                let hcl::Value::Object(next) = entry else { panic!("{path} crosses a value") };
                Self::insert(next, rest, value);
            }
        }
    }
}

#[test]
fn descriptor_passes_the_manager_checker() {
    let descriptor = Fixture::descriptor();

    assert_eq!(descriptor.namespace, "alaydriem");
    assert_eq!(descriptor.name, "freecam");
    assert_eq!(descriptor.id(), "@alaydriem/freecam");
    assert_eq!(descriptor.config_path, "config.hcl");
    assert!(descriptor.claims.iter().all(|claim| claim.mode == ClaimMode::Shared));
    let blocking: Vec<_> =
        descriptor.claims.iter().filter(|claim| claim.block_player_turn).map(|claim| claim.capability).collect();
    assert_eq!(blocking, [Capability::Input]);
    let hiding: Vec<_> =
        descriptor.claims.iter().filter(|claim| claim.hide_vanilla_hud).map(|claim| claim.capability).collect();
    assert_eq!(hiding, [Capability::Hud]);
    assert_eq!(descriptor.input_passthrough, [27, 112]);
}

#[test]
fn only_toggle_is_global() {
    let descriptor = Fixture::descriptor();

    let global: Vec<_> = descriptor
        .keybinds
        .iter()
        .filter(|keybind| keybind.scope == KeyScope::Global)
        .map(|keybind| keybind.action.as_str())
        .collect();

    assert_eq!(global, ["toggle"]);
}

#[test]
fn keybind_order_matches_freecam_action() {
    let descriptor = Fixture::descriptor();

    for (index, keybind) in descriptor.keybinds.iter().enumerate() {
        let index = u16::try_from(index).unwrap();
        assert_eq!(FreecamAction::from_index(index).map(FreecamAction::name), Some(keybind.action.as_str()));
    }
    assert_eq!(FreecamAction::from_index(u16::try_from(descriptor.keybinds.len()).unwrap()), None);
}

#[test]
fn schema_defaults_match_config_defaults() {
    let descriptor = Fixture::descriptor();
    let mut root = hcl::Map::new();
    for field in descriptor.settings.iter().flat_map(|section| &section.fields) {
        Fixture::insert(&mut root, &field.path, field.default.clone());
    }
    let text = hcl::to_string(&hcl::Value::Object(root)).unwrap();

    let config: FreecamConfig = hcl::from_str(&text).unwrap();

    assert_eq!(config, FreecamConfig::default());
}
