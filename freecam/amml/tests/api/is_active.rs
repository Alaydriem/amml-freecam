use alaydriem_freecam::api::is_active::alaydriem_freecam_is_active;

use crate::support::PeExports;

#[test]
fn is_active_is_false_before_the_mod_loads() {
    assert!(!alaydriem_freecam_is_active());
}

#[test]
fn the_built_dll_exports_is_active() {
    // the integration test runs from target/<profile>/deps, where cargo also writes the cdylib
    let deps = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let dll = deps.join("alaydriem_freecam.dll");
    assert!(dll.exists(), "{} is not built", dll.display());

    let names = PeExports::names(&dll);

    assert!(names.iter().any(|name| name == "alaydriem_freecam_is_active"), "{names:?}");
}
