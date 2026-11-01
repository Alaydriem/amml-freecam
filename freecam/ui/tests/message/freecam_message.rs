use std::path::Path;

use amml_i18n_tool::catalog::CatalogCheck;
use freecam_ui::message::FreecamMessage;

#[test]
fn message_catalog() {
    let problems: Vec<String> = CatalogCheck::problems::<FreecamMessage>().iter().map(ToString::to_string).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn pot_up_to_date() {
    let i18n = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let current = CatalogCheck::pot_is_current::<FreecamMessage>(&i18n).unwrap_or_else(|error| panic!("{error}"));
    assert!(current, "freecam.pot is out of date: run `mise run i18n:extract`");
}
