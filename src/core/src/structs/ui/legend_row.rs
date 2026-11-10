#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegendRow {
    pub key: String,
    pub label: String,
    // shown in place of the label while the cursor is on
    pub label_cursor_on: Option<String>,
    // the row shows only while the cursor is on
    pub cursor_only: bool,
}
