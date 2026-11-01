use crate::structs::path::PlaybackStep;

pub const EASE_RAMP_SECONDS: f32 = 1.0;
// halvings that find a playback time from a distance, far below a microsecond on any real path
const SEARCH_STEPS: u32 = 60;

pub struct PathPlayer {
    ramp_seconds: f32,
    running: bool,
    length: f64,
    // the distance this playback travels, from the start distance to the end
    span: f64,
    speed: f64,
    // ramp time after shrinking so both ramps fit in the span
    ramp: f64,
    total_seconds: f64,
    settle_remaining: u32,
    // true once the settle is over and the camera travels
    moving: bool,
    elapsed: f64,
    // where the playback started, held during the settle
    start_distance: f64,
    held_distance: f64,
}

impl PathPlayer {
    pub fn new(ramp_seconds: f32) -> Self {
        Self {
            ramp_seconds: ramp_seconds.max(0.0),
            running: false,
            length: 0.0,
            span: 0.0,
            speed: 0.0,
            ramp: 0.0,
            total_seconds: 0.0,
            settle_remaining: 0,
            moving: false,
            elapsed: 0.0,
            start_distance: 0.0,
            held_distance: 0.0,
        }
    }

    // starts at `from` blocks along the path and eases in from rest there, with the same ramps as a
    // playback from the start
    pub fn start(&mut self, length: f32, speed: f32, settle_frames: u32, from: f32) {
        self.running = true;
        self.length = f64::from(length.max(0.0));
        self.speed = f64::from(speed.max(0.0));
        self.settle_remaining = settle_frames;
        self.moving = false;
        self.elapsed = 0.0;
        self.start_distance = 0.0;
        self.held_distance = 0.0;
        if self.speed <= 0.0 || self.length <= 0.0 {
            self.length = 0.0;
            self.span = 0.0;
            self.ramp = 0.0;
            self.total_seconds = 0.0;
            return;
        }
        let from = if from.is_finite() { f64::from(from).clamp(0.0, self.length) } else { 0.0 };
        self.start_distance = from;
        self.held_distance = from;
        self.span = self.length - from;
        self.ramp = f64::from(self.ramp_seconds).min(self.span / self.speed);
        self.total_seconds = self.span / self.speed + self.ramp;
    }

    // the seconds a playback takes: travel time plus the ramp, which shrinks on a short path
    pub fn duration(length: f32, speed: f32, ramp_seconds: f32) -> f32 {
        if speed <= 0.0 || length <= 0.0 {
            return 0.0;
        }
        let travel = length / speed;
        travel + ramp_seconds.max(0.0).min(travel)
    }

    // the playback time at which a playback from the start reaches `distance`, ramps included; 0
    // when the path cannot play
    pub fn seconds_at(length: f32, speed: f32, ramp_seconds: f32, distance: f32) -> f32 {
        if speed <= 0.0 || length <= 0.0 || !distance.is_finite() {
            return 0.0;
        }
        let (length, speed) = (f64::from(length), f64::from(speed));
        let ramp = f64::from(ramp_seconds.max(0.0)).min(length / speed);
        let total = length / speed + ramp;
        Self::seconds_for(length, speed, ramp, total, f64::from(distance).clamp(0.0, length)) as f32
    }

    // keeps the playback in its settle for one more step when the settle has not ended yet
    pub fn hold_settle(&mut self) {
        if self.running && !self.moving {
            self.settle_remaining = self.settle_remaining.max(1);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        if self.running {
            self.held_distance = self.current_distance();
        }
        self.running = false;
    }

    pub fn step(&mut self, dt: f64, paused: bool) -> PlaybackStep {
        if !self.running {
            return PlaybackStep { distance: self.held_distance as f32, settling: false, finished: true };
        }
        if paused {
            let settling = self.settle_remaining > 0;
            return PlaybackStep { distance: self.current_distance() as f32, settling, finished: false };
        }
        if self.settle_remaining > 0 {
            self.settle_remaining -= 1;
            return PlaybackStep { distance: self.start_distance as f32, settling: true, finished: false };
        }
        self.moving = true;
        self.elapsed += dt.max(0.0);
        if self.elapsed >= self.total_seconds {
            self.running = false;
            self.held_distance = self.length;
            return PlaybackStep { distance: self.length as f32, settling: false, finished: true };
        }
        PlaybackStep { distance: self.distance_at(self.elapsed) as f32, settling: false, finished: false }
    }

    // the start distance during the settle, else the distance at the elapsed time
    fn current_distance(&self) -> f64 {
        if self.settle_remaining > 0 { self.start_distance } else { self.distance_at(self.elapsed) }
    }

    fn distance_at(&self, seconds: f64) -> f64 {
        let travelled = Self::distance_for(self.span, self.speed, self.ramp, self.total_seconds, seconds);
        (self.start_distance + travelled).min(self.length)
    }

    fn distance_for(length: f64, speed: f64, ramp: f64, total: f64, seconds: f64) -> f64 {
        if total <= 0.0 {
            return 0.0;
        }
        let ramp_distance = speed * ramp / 2.0;
        let down_start = total - ramp;
        let distance = if seconds < ramp {
            Self::ramp_distance(speed, ramp, seconds / ramp)
        } else if seconds < down_start {
            ramp_distance + speed * (seconds - ramp)
        } else {
            let remaining = (total - seconds) / ramp;
            length - Self::ramp_distance(speed, ramp, remaining)
        };
        distance.clamp(0.0, length)
    }

    // the distance only grows with time, so halving the time range finds the time of a distance
    fn seconds_for(length: f64, speed: f64, ramp: f64, total: f64, distance: f64) -> f64 {
        if distance <= 0.0 {
            return 0.0;
        }
        if distance >= length {
            return total;
        }
        let (mut low, mut high) = (0.0, total);
        for _ in 0..SEARCH_STEPS {
            let middle = (low + high) / 2.0;
            if Self::distance_for(length, speed, ramp, total, middle) < distance {
                low = middle;
            } else {
                high = middle;
            }
        }
        (low + high) / 2.0
    }

    fn ramp_distance(speed: f64, ramp: f64, fraction: f64) -> f64 {
        speed * ramp * (fraction.powi(3) - fraction.powi(4) / 2.0)
    }
}
