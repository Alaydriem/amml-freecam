use freecam_core::config::KeyConfig;
use freecam_core::structs::UiLegend;

// the rows shown in this mode, as key and label
fn shown(legend: &UiLegend, cursor_mode: bool) -> Vec<(&str, &str)> {
    (0..legend.rows.len())
        .filter(|row| legend.shown(*row, cursor_mode))
        .map(|row| (legend.rows[row].key.as_str(), legend.label(row, cursor_mode)))
        .collect()
}

#[test]
fn legend_has_r3_rows() {
    let legend = UiLegend::new(&KeyConfig::default());

    assert_eq!(
        shown(&legend, false),
        [("F3+E", "show bar"), ("Ctrl+B", "add point"), ("F3+Delete", "delete point"), ("F3+P", "exit freecam")]
    );
}

#[test]
fn bar_mode_adds_the_drag_row_and_relabels_the_bar_row() {
    let legend = UiLegend::new(&KeyConfig::default());

    assert_eq!(
        shown(&legend, true),
        [
            ("F3+E", "hide bar"),
            ("RMB drag", "look"),
            ("Ctrl+B", "add point"),
            ("F3+Delete", "delete point"),
            ("F3+P", "exit freecam")
        ]
    );
}

#[test]
fn a_row_past_the_end_is_not_shown() {
    let legend = UiLegend::new(&KeyConfig::default());

    assert!(!legend.shown(legend.rows.len(), true));
    assert_eq!(legend.label(legend.rows.len(), true), "");
}

#[test]
fn legend_shows_bound_chords() {
    let keys = KeyConfig { toggle: "Ctrl+Shift+F10".to_owned(), add_point: "N".to_owned(), ..KeyConfig::default() };

    let legend = UiLegend::new(&keys);

    let shown = shown(&legend, false);
    assert_eq!(shown[1].0, "N");
    assert_eq!(shown[3].0, "Ctrl+Shift+F10");
}
