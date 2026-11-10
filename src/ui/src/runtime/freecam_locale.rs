use std::sync::Arc;

use amml_i18n::localize::Localizer;
use amml_i18n::message::Message;
use amml_i18n::service::{LocaleSetting, LocalizationService};
use amml_ui::AmmlUiMessage;
use amml_ui::text::AmmlUiPacks;

use crate::message::FreecamMessage;
use crate::text::FreecamPacks;

/// Freecam's words in one language: its own packs and the amml-ui widgets' packs.
pub struct FreecamLocale;

impl FreecamLocale {
    /// `code` is the manager's language code: empty for English, `en_XA` for the pseudo-locale.
    pub fn localizer(code: &str) -> Arc<Localizer> {
        let service = LocalizationService::new(vec![
            (AmmlUiMessage::DOMAIN, AmmlUiPacks::SOURCE),
            (FreecamMessage::DOMAIN, FreecamPacks::SOURCE),
        ]);
        Arc::new(service.load(&LocaleSetting::from_code(code)))
    }
}
