use crate::structs::{CaptureRequest, ScreenshotStep};

// presented frames a capture request may wait for its frame to be taken, counted from the request; the mod manager
// gives up on a copy after 30 frames and answers then, so freecam waits longer than that
pub const CAPTURE_TIMEOUT_FRAMES: u32 = 40;
// new presents after the markers left before the capture is asked for; the frame call and Present run pipelined, so
// the present in flight when the last removes went out can still hold the markers
pub const SETTLE_PRESENTS: u32 = 2;
// seconds of frame time a taken shot may wait for its written answer
pub const WRITE_TIMEOUT_SECONDS: f64 = 10.0;

// hides the mod's drawing, waits for the markers to leave the world, asks for one capture and gives the
// drawing back when the frame is taken or the wait runs out; the written answer comes later and names the file
//
// Every frame count is in presented frames: a step advances only when the present count changes, so the
// several the frame call calls the game makes for one presented frame count once.
//
// Bound: at most one shot runs at a time, and a new one is refused until the last taken shot is written. The
// wait for markers lasts `wait_limit` presents, and a shot whose markers are still live then is skipped. The
// wait for the taken answer lasts `CAPTURE_TIMEOUT_FRAMES` presents and the wait for the written answer
// `WRITE_TIMEOUT_SECONDS` of frame time; a written answer after that is ignored.
pub struct ScreenshotSequence {
    wait_limit: u32,
    sequence: u32,
    active: bool,
    // true once the markers are gone, or the wait for them ran out
    settled: bool,
    capturing: bool,
    // presents spent in the current wait
    frames: u32,
    // the present count of the last step, None before the first
    last_present: Option<u32>,
    // the taken shot whose PNG is still being written
    writing: Option<u32>,
    // seconds the written answer may still take
    writing_left: f64,
}

impl ScreenshotSequence {
    pub fn new(wait_limit: u32) -> Self {
        Self {
            wait_limit,
            sequence: 0,
            active: false,
            settled: false,
            capturing: false,
            frames: 0,
            last_present: None,
            writing: None,
            writing_left: 0.0,
        }
    }

    // starts a shot; false when one is running or the last one is still being written
    pub fn request(&mut self) -> bool {
        if self.active || self.writing.is_some() {
            return false;
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.active = true;
        self.settled = false;
        self.capturing = false;
        self.frames = 0;
        true
    }

    // true while the UI and the markers must stay out of the picture
    pub fn hides(&self) -> bool {
        self.active
    }

    // drops the shot before it is taken, without a result
    pub fn abort(&mut self) {
        self.active = false;
        self.capturing = false;
    }

    // runs once per the frame call; `live_slots` is the marker count still in the world, `present_count` the
    // count of frames the mod manager drew and `dt` the frame time in seconds. The markers may settle on any call;
    // every other change waits for a new present, so the capture is asked for `SETTLE_PRESENTS` presented
    // frames after the markers left
    pub fn step(&mut self, live_slots: usize, present_count: u32, dt: f64) -> ScreenshotStep {
        let new_present = self.last_present != Some(present_count);
        self.last_present = Some(present_count);
        if self.writing.is_some() {
            self.writing_left -= dt;
            if self.writing_left <= 0.0 {
                self.writing = None;
                return ScreenshotStep::WriteTimedOut;
            }
        }
        if !self.active {
            return ScreenshotStep::Waiting;
        }
        if self.capturing {
            if new_present {
                self.frames = self.frames.saturating_add(1);
                if self.frames >= CAPTURE_TIMEOUT_FRAMES {
                    self.abort();
                    return ScreenshotStep::TimedOut;
                }
            }
            return ScreenshotStep::Waiting;
        }
        if self.settled {
            if !new_present {
                return ScreenshotStep::Waiting;
            }
            self.frames = self.frames.saturating_add(1);
            if self.frames < SETTLE_PRESENTS {
                return ScreenshotStep::Waiting;
            }
            self.capturing = true;
            self.frames = 0;
            return ScreenshotStep::Capture(CaptureRequest { sequence: self.sequence });
        }
        if new_present {
            self.frames = self.frames.saturating_add(1);
        }
        self.settled = live_slots == 0;
        if self.settled {
            self.frames = 0;
        } else if self.frames >= self.wait_limit {
            self.abort();
            return ScreenshotStep::MarkersStuck;
        }
        ScreenshotStep::Waiting
    }

    // the frame of the waiting request was copied (`ok`) or could not be; the drawing may come back. A copied
    // frame waits for its written answer. False when it is not the answer to the waiting request
    pub fn taken(&mut self, sequence: u32, ok: bool) -> bool {
        if !self.capturing || sequence != self.sequence {
            return false;
        }
        self.abort();
        if ok {
            self.writing = Some(sequence);
            self.writing_left = WRITE_TIMEOUT_SECONDS;
        }
        true
    }

    // the PNG of a taken shot was written or failed; false when no taken shot with this sequence waits for it
    pub fn written(&mut self, sequence: u32) -> bool {
        if self.writing != Some(sequence) {
            return false;
        }
        self.writing = None;
        true
    }
}
