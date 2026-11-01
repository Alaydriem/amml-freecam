use amml_i18n::message::Message;
use amml_i18n::text::Text;
use freecam_ui::message::FreecamMessage;
use freecam_ui::runtime::FreecamLocale;

#[test]
fn pseudo_locale_changes_every_word() {
    let localizer = FreecamLocale::localizer("en_XA");
    for message in FreecamMessage::ALL.iter().filter(|message| !message.msgid().contains('{')) {
        let resolved = Text::message(*message).resolve(&localizer).into_owned();
        assert_ne!(resolved, message.msgid(), "{message:?} has no pseudo form");
    }
}

#[test]
fn empty_code_is_english() {
    let localizer = FreecamLocale::localizer("");
    assert_eq!(Text::message(FreecamMessage::AddPoint).resolve(&localizer), "Add point");
}
