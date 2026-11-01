use std::fmt;

/// Timecode text for short freecam shots.
pub struct Timecode;

impl Timecode {
    /// `MM:SS.cc`, rounded to the nearest hundredth; a negative or NaN time shows as zero.
    pub fn short(seconds: f32) -> String {
        let mut text = String::new();
        if Self::write_short(&mut text, seconds).is_err() {
            text.clear();
        }
        text
    }

    /// Writes [`Self::short`] into `out` without allocating.
    pub fn write_short(out: &mut impl fmt::Write, seconds: f32) -> fmt::Result {
        let centis = (seconds * 100.0).round().max(0.0) as u32;
        write!(out, "{:02}:{:02}.{:02}", centis / 6000, (centis / 100) % 60, centis % 100)
    }
}
