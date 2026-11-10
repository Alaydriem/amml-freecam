#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MarkerKind {
    // removes the shape in the slot; only the id is used
    #[default]
    Remove,
    Line,
    Box,
    Arrow,
}
