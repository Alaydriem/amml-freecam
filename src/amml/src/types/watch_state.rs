// what the camera watch keeps between frames: the server camera flag from the server's camera instructions, the
// capture generation it belongs to, and when the registry read and the flag began to disagree
#[derive(Debug, Clone, Copy, Default)]
pub struct WatchState {
    pub server_camera: bool,
    pub generation: u32,
    pub disagree_since: Option<f64>,
    pub disagree_logged: bool,
}
