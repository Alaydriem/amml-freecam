use amml_i18n::message::{Domain, Message};

/// The words freecam paints. The legend labels come from freecam-core and stay English for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreecamMessage {
    Title,
    PathSummary,
    ScreenshotFailed,
    ScreenshotSaved,
    ClearPoints,
    ClickAgainToClear,
    Speed,
    SpeedValue,
    AddPoint,
    PointCount,
    AddsAtPlayhead,
    Replace,
    Delete,
    PointTag,
    NothingToClear,
    ScreenshotUnavailable,
}

impl Message for FreecamMessage {
    const DOMAIN: Domain = Domain::new("freecam");
    const ALL: &'static [Self] = &[
        Self::Title,
        Self::PathSummary,
        Self::ScreenshotFailed,
        Self::ScreenshotSaved,
        Self::ClearPoints,
        Self::ClickAgainToClear,
        Self::Speed,
        Self::SpeedValue,
        Self::AddPoint,
        Self::PointCount,
        Self::AddsAtPlayhead,
        Self::Replace,
        Self::Delete,
        Self::PointTag,
        Self::NothingToClear,
        Self::ScreenshotUnavailable,
    ];

    fn index(self) -> usize {
        self as usize
    }

    fn msgid(self) -> &'static str {
        match self {
            Self::Title => "FREECAM",
            Self::PathSummary => "PATH  \u{b7}  {count} point  \u{b7}  {seconds} s",
            Self::ScreenshotFailed => "Screenshot failed",
            Self::ScreenshotSaved => "Screenshot saved: {name}",
            Self::ClearPoints => "Clear points",
            Self::ClickAgainToClear => "Click again to clear",
            Self::Speed => "SPEED",
            Self::SpeedValue => "{speed} b/s",
            Self::AddPoint => "Add point",
            Self::PointCount => "{count} point",
            Self::AddsAtPlayhead => "Adds at the playhead",
            Self::Replace => "Re-place",
            Self::Delete => "Delete",
            Self::PointTag => "Point {number}",
            Self::NothingToClear => "There are no points to clear",
            Self::ScreenshotUnavailable => "A screenshot cannot be taken now",
        }
    }

    fn context(self) -> Option<&'static str> {
        None
    }

    fn plural(self) -> Option<&'static str> {
        match self {
            Self::PathSummary => Some("PATH  \u{b7}  {count} points  \u{b7}  {seconds} s"),
            Self::PointCount => Some("{count} points"),
            _ => None,
        }
    }

    fn note(self) -> Option<&'static str> {
        match self {
            Self::Title => Some("The title of the freecam strip, painted in spaced capitals"),
            Self::PathSummary => Some("The freecam strip: how many path points, then the path length in seconds"),
            Self::ScreenshotSaved => Some("{name} is the file name of the screenshot"),
            Self::Speed => Some("The caption of the speed chip, painted in spaced capitals"),
            Self::SpeedValue => Some("b/s is blocks per second"),
            Self::Replace => Some("Puts the selected path point again at the camera"),
            Self::PointTag => Some("The selected path point, as in Point 3"),
            _ => None,
        }
    }
}
