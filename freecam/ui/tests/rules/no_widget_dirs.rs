use amml_ui_rules::folder::ForbiddenFolders;

#[test]
fn no_widget_dirs() {
    // freecam-ui builds screens from amml-ui widgets and tokens; it keeps no widget or theme code of its own.
    let present = ForbiddenFolders::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"), &["widgets", "theme"]).present();
    assert!(present.is_empty(), "freecam-ui has widget or theme code: {present:?}");
}
