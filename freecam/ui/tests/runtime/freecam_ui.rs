use amml_i18n::message::Message;
use amml_ui::font::FontLoader;
use egui::FontFamily;
use freecam_core::structs::{ToastKind, UiAction, UiLegend};
use freecam_ui::message::FreecamMessage;
use freecam_ui::views::BarLayout;

use crate::support::ui_fixture::{UiEvent, UiFixture};

fn click_at(x: f32, y: f32) -> [UiEvent; 3] {
    [
        UiEvent::PointerMoved { x, y },
        UiEvent::PointerButton { x, y, button: 0, pressed: true },
        UiEvent::PointerButton { x, y, button: 0, pressed: false },
    ]
}

fn layout() -> BarLayout {
    BarLayout::new(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(UiFixture::SCREEN[0], UiFixture::SCREEN[1])))
}

#[test]
fn show_draws_nothing_when_the_snapshot_is_hidden() {
    let mut fixture = UiFixture::new();
    fixture.snapshot.visible = false;

    let out = fixture.frame(&[]);

    assert!(out.rect.is_none());
    assert!(out.vertices.is_empty());
}

#[test]
fn fly_mode_draws_strip_and_legend_only() {
    let mut fixture = UiFixture::new();
    fixture.settle();

    let out = fixture.frame(&[]);

    let rect = out.rect.unwrap();
    assert!(rect.bottom() < layout().bar.top(), "{rect:?}");
    let lowest = out.vertices.iter().map(|v| v.y).fold(0.0_f32, f32::max);
    assert!(lowest < layout().bar.top(), "{lowest}");
}

#[test]
fn bar_mode_reaches_the_bar() {
    let mut fixture = UiFixture::new().with_path();
    fixture.settle();

    let rect = fixture.frame(&[]).rect.unwrap();

    assert!(rect.bottom() >= layout().bar.bottom());
}

#[test]
fn screenshot_button_hidden_when_capture_unavailable() {
    let shot = layout().shot.center();
    let mut available = UiFixture::new().with_path();
    available.settle();
    let mut unavailable = UiFixture::new().with_path();
    unavailable.snapshot.can_screenshot = false;
    unavailable.settle();

    let clicked = available.frame(&click_at(shot.x, shot.y)).actions.clone();
    let refused = unavailable.frame(&click_at(shot.x, shot.y)).actions.clone();

    assert_eq!(clicked, [UiAction::Screenshot]);
    assert!(refused.is_empty(), "{refused:?}");
}

#[test]
fn uses_studio_font() {
    let mut fixture = UiFixture::new();
    fixture.frame(&[]);

    let fonts = fixture.ctx.fonts(|fonts| fonts.definitions().clone());

    assert_eq!(fonts.families[&FontFamily::Proportional].first().map(String::as_str), Some(FontLoader::TEXT_KEY));
}

#[test]
fn textures_lost_keeps_drawing_the_same_picture() {
    let mut fixture = UiFixture::new().with_path();
    fixture.settle();
    let before = fixture.frame(&[]).vertices.len();

    fixture.ui.textures_lost();
    let after = fixture.frame(&[]).vertices.len();

    assert_eq!(before, after);
}

#[test]
fn bar_wants_the_pointer_over_the_bar_only() {
    let mut fixture = UiFixture::new().with_path();
    fixture.settle();
    let bar = layout().bar;
    let wants_at = |fixture: &mut UiFixture, x: f32, y: f32| {
        fixture.frame(&[UiEvent::PointerMoved { x, y }]);
        fixture.ctx.egui_wants_pointer_input()
    };

    // the mod manager hands a click to a layer only while egui wants the pointer there
    let over_bar = wants_at(&mut fixture, bar.center().x, bar.center().y);
    let away = wants_at(&mut fixture, bar.center().x, bar.top() - 300.0);

    assert!(over_bar);
    assert!(!away);
}

// the bar with three points in the pseudo-locale: idle with clear armed, or with point 2 selected; a saved toast
fn pseudo_bar(selection: Option<usize>) -> UiFixture {
    let mut fixture = UiFixture::new().with_path().locale("en_XA");
    fixture.snapshot.selection = selection;
    fixture.snapshot.clear_armed = selection.is_none();
    fixture.snapshot.toast = ToastKind::Saved;
    fixture.snapshot.toast_file[..8].copy_from_slice(b"shot.png");
    fixture.snapshot.toast_file_len = 8;
    fixture.settle();
    fixture
}

// the legend's words, which freecam-core builds and which stay English
fn legend_words(legend: &UiLegend) -> Vec<String> {
    let mut words = Vec::new();
    for (index, row) in legend.rows.iter().enumerate() {
        words.push(row.key.clone());
        words.push(legend.label(index, false).to_owned());
        words.push(legend.label(index, true).to_owned());
    }
    words
}

#[test]
fn bar_paints_no_english_word_in_pseudo_locale() {
    let english: Vec<&str> = FreecamMessage::ALL
        .iter()
        .flat_map(|message| message.msgid().split(|c: char| !c.is_ascii_alphabetic()))
        .filter(|word| word.len() >= 3 && !["count", "seconds", "name", "speed", "number"].contains(word))
        .collect();
    for selection in [None, Some(1)] {
        let mut fixture = pseudo_bar(selection);
        let allowed = legend_words(&fixture.legend);
        let texts = fixture.texts();
        for (text, _) in texts.iter().filter(|(text, _)| !allowed.contains(text)) {
            let word = english.iter().find(|word| text.contains(**word));
            assert!(word.is_none(), "selection {selection:?}: {text:?} has the English word {word:?}");
        }
    }
}

#[test]
fn bar_paints_inside_its_rect_in_pseudo_locale() {
    let layout = layout();
    for selection in [None, Some(1)] {
        let mut fixture = pseudo_bar(selection);
        let texts = fixture.texts();
        let on_bar: Vec<_> = texts.iter().filter(|(_, rect)| layout.bar.contains(rect.left_top())).collect();
        for (text, rect) in &on_bar {
            assert!(
                rect.right() <= layout.bar.right() + 0.5,
                "selection {selection:?}: {text:?} {rect:?} leaves the bar"
            );
            // the speed chip and, with a selection, the point tag are fixed parts: their words stay in them
            let parts = if selection.is_some() { vec![layout.speed, layout.tag] } else { vec![layout.speed] };
            for part in parts {
                if part.contains(rect.left_top()) {
                    assert!(
                        rect.right() <= part.right() + 0.5,
                        "selection {selection:?}: {text:?} {rect:?} leaves {part:?}"
                    );
                }
            }
        }
        for (index, (text, rect)) in on_bar.iter().enumerate() {
            for (other, other_rect) in &on_bar[index + 1..] {
                let overlap = rect.shrink(0.5).intersect(other_rect.shrink(0.5));
                assert!(
                    !overlap.is_positive(),
                    "selection {selection:?}: {text:?} {rect:?} overlaps {other:?} {other_rect:?}"
                );
            }
        }
    }
}
