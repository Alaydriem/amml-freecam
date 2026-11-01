// what the user did in the UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiAction {
    // starts playback from the playhead
    Play,
    // playhead and camera to point 1, selection cleared
    Rewind,
    // playhead and camera to this distance along the path
    Scrub(f32),
    // selects this point, or clears the selection when it is already selected
    SelectPoint(usize),
    // the selected point takes the current camera pose
    Replace,
    // removes the selected point
    Delete,
    // inserts the current camera pose at the playhead
    AddPoint,
    // the first click arms the clear, a second within the arm time removes every point
    ClearPoints,
    Screenshot,
}
