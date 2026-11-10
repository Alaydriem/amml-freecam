use crate::config::KeyConfig;
use crate::structs::ui::LegendRow;

// the key legend, built from the key config when it loads or changes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiLegend {
    pub rows: Vec<LegendRow>,
}

impl UiLegend {
    pub fn new(keys: &KeyConfig) -> Self {
        let row = |key: &str, label: &str, label_cursor_on: Option<&str>| LegendRow {
            key: key.to_owned(),
            label: label.to_owned(),
            label_cursor_on: label_cursor_on.map(str::to_owned),
            cursor_only: false,
        };
        let drag = LegendRow { cursor_only: true, ..row("RMB drag", "look", None) };
        Self {
            rows: vec![
                row(&keys.toggle_bar, "show bar", Some("hide bar")),
                drag,
                row(&keys.add_point, "add point", None),
                row(&keys.delete_point, "delete point", None),
                row(&keys.toggle, "exit freecam", None),
            ],
        }
    }

    // false for a row that does not exist
    pub fn shown(&self, row: usize, cursor_mode: bool) -> bool {
        self.rows.get(row).is_some_and(|row| cursor_mode || !row.cursor_only)
    }

    // an empty string for a row that does not exist
    pub fn label(&self, row: usize, cursor_mode: bool) -> &str {
        let Some(row) = self.rows.get(row) else {
            return "";
        };
        match (&row.label_cursor_on, cursor_mode) {
            (Some(label), true) => label,
            _ => &row.label,
        }
    }
}
