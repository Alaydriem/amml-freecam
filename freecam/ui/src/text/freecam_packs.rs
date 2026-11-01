use amml_i18n::service::PackSource;

/// The compiled language packs of `FreecamMessage`, built into the freecam DLL. Each file in `i18n/packs/` is
/// listed here; a test fails when one is missing.
pub struct FreecamPacks;

impl FreecamPacks {
    pub const SOURCE: PackSource = PackSource::Embedded(&[("en_XA", include_bytes!("../../i18n/packs/en_XA.json"))]);
}
