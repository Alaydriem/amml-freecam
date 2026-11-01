use crate::structs::frame::CaptureRequest;

// what the screenshot sequence asks of the controller this frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotStep {
    Waiting,
    Capture(CaptureRequest),
    // the capture went unanswered for too long and the sequence is over
    TimedOut,
    // the markers did not leave the world within the wait, so no capture was asked for
    MarkersStuck,
    // a taken shot got no written answer in time; new shots may start
    WriteTimedOut,
}
