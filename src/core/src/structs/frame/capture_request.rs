// asks the mod layer to capture the frame presented now
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaptureRequest {
    // echoed back in the capture result
    pub sequence: u32,
}
