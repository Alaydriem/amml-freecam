use crate::services::PathCurve;

// a point this close to the playhead, in blocks, counts as at the playhead
const AT_PLAYHEAD: f32 = 1e-3;

// the playback bar's selected point and its playhead, a distance along the position curve
pub struct BarState {
    selection: Option<usize>,
    playhead: f32,
}

impl BarState {
    pub fn new() -> Self {
        Self { selection: None, playhead: 0.0 }
    }

    pub fn selection(&self) -> Option<usize> {
        self.selection
    }

    pub fn playhead(&self) -> f32 {
        self.playhead
    }

    // where a new key goes: after the last of the `count` points at or before the playhead; 0 on an
    // empty path
    pub fn insert_index(&self, curve: &PathCurve, count: usize) -> usize {
        (0..count).rev().find(|index| curve.point_distance(*index) <= self.playhead + AT_PLAYHEAD).map_or(0, |i| i + 1)
    }

    // a key went in at `index` at `distance`; the selected point stays the same point
    pub fn on_inserted(&mut self, index: usize, distance: f32) {
        if let Some(selected) = self.selection
            && selected >= index
        {
            self.selection = Some(selected + 1);
        }
        self.playhead = distance;
    }

    // a click on point `index` at `distance`: selects it and moves the playhead there, or clears the
    // selection when it was already selected; true when the camera must move to the point
    pub fn select(&mut self, index: usize, distance: f32) -> bool {
        if self.selection == Some(index) {
            self.selection = None;
            return false;
        }
        self.selection = Some(index);
        self.playhead = distance;
        true
    }

    // moves the playhead inside the path of this length; the selection stays
    pub fn scrub(&mut self, distance: f32, length: f32) {
        self.playhead = Self::inside(distance, length);
    }

    pub fn set_playhead(&mut self, distance: f32) {
        self.playhead = distance;
    }

    pub fn rewind(&mut self) {
        self.selection = None;
        self.playhead = 0.0;
    }

    // the selected key was removed; the playhead goes to `distance`
    pub fn on_deleted(&mut self, distance: f32) {
        self.selection = None;
        self.playhead = distance;
    }

    // the distance a playback starts from: the playhead, or the start when the playhead is at the end
    pub fn play_from(&self, length: f32) -> f32 {
        if self.playhead >= length - AT_PLAYHEAD { 0.0 } else { self.playhead }
    }

    // keeps the selection and the playhead inside a path of `count` points and this length
    pub fn clamp(&mut self, count: usize, length: f32) {
        if self.selection.is_some_and(|selected| selected >= count) {
            self.selection = None;
        }
        self.playhead = Self::inside(self.playhead, length);
    }

    fn inside(distance: f32, length: f32) -> f32 {
        let length = if length.is_finite() { length.max(0.0) } else { 0.0 };
        if distance.is_nan() { 0.0 } else { distance.clamp(0.0, length) }
    }
}

impl Default for BarState {
    fn default() -> Self {
        Self::new()
    }
}
