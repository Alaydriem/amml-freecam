// the message the bar's toast shows
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastKind {
    #[default]
    None,
    // "Screenshot saved: <file name>"
    Saved,
    // "Screenshot failed"
    Failed,
}
