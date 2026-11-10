use crate::structs::{FrameStatus, TelemetryReport, UiTiming, UiTimingSummary, Vec3};

// seconds of recorded frame time per report
const WINDOW_SECONDS: f32 = 2.0;
// absorbs float rounding when frame times add up to exactly one window
const WINDOW_TOLERANCE: f32 = 1e-3;
// slower frames are not steady enough to judge step size
const MIN_STEADY_SPEED: f32 = 0.5;
// largest speed change between frames that still counts as steady
const STEADY_SPEED_TOLERANCE: f32 = 0.02;

pub struct SmoothnessTelemetry {
    enabled: bool,
    elapsed: f32,
    frames: u32,
    min_dt: f32,
    max_dt: f32,
    step_ratio_min: f32,
    step_ratio_max: f32,
    aim_turn_max_deg: f32,
    step_samples: u32,
    sets: u32,
    camera_updates: u32,
    max_drift_degrees: f32,
    // the last rendered position seen, kept across frames that report none
    previous_rendered: Option<Vec3>,
    previous_speed: Option<f32>,
    // expected travel of the frames since the last rendered position, and whether all were steady
    expected_since_rendered: f32,
    steady_since_rendered: bool,
    missed_overrides: u32,
    previous_camera: Option<Vec3>,
    report: Option<TelemetryReport>,
    ui_cpu_sum: f32,
    ui_cpu_max: f32,
    ui_cpu_samples: u32,
    ui_gpu_sum: f32,
    ui_gpu_max: f32,
    ui_gpu_samples: u32,
    ui_last: UiTiming,
    marker_live_slots: u32,
    marker_entries_sent: u32,
    marker_max_send_entries: u32,
    marker_max_send_us: f32,
    marker_reductions: u32,
    path_rebuild_us_max: f32,
    look_at_rays_answered: u32,
    look_at_rays_timed_out: u32,
    // the time of the first flight key-down not yet followed by a camera move
    key_probe: Option<f64>,
    // key-to-move times of the open window in milliseconds; when full, the oldest is overwritten
    key_samples: [f32; Self::KEY_PROBE_CAPACITY],
    key_sample_count: usize,
    key_sample_next: usize,
}

impl SmoothnessTelemetry {
    pub const KEY_PROBE_CAPACITY: usize = 128;

    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            elapsed: 0.0,
            frames: 0,
            min_dt: f32::MAX,
            max_dt: 0.0,
            step_ratio_min: f32::MAX,
            step_ratio_max: 0.0,
            aim_turn_max_deg: 0.0,
            step_samples: 0,
            sets: 0,
            camera_updates: 0,
            max_drift_degrees: 0.0,
            previous_rendered: None,
            previous_speed: None,
            expected_since_rendered: 0.0,
            steady_since_rendered: false,
            missed_overrides: 0,
            previous_camera: None,
            report: None,
            ui_cpu_sum: 0.0,
            ui_cpu_max: 0.0,
            ui_cpu_samples: 0,
            ui_gpu_sum: 0.0,
            ui_gpu_max: 0.0,
            ui_gpu_samples: 0,
            ui_last: UiTiming::default(),
            marker_live_slots: 0,
            marker_entries_sent: 0,
            marker_max_send_entries: 0,
            marker_max_send_us: 0.0,
            marker_reductions: 0,
            path_rebuild_us_max: 0.0,
            look_at_rays_answered: 0,
            look_at_rays_timed_out: 0,
            key_probe: None,
            key_samples: [0.0; Self::KEY_PROBE_CAPACITY],
            key_sample_count: 0,
            key_sample_next: 0,
        }
    }

    // forgets the previous frame and the open window, for a new freecam session
    pub fn restart(&mut self) {
        *self = Self::new(self.enabled);
    }

    pub fn record(&mut self, dt: f32, status: &FrameStatus, sent_set: bool, velocity: Vec3, view_forward: Vec3) {
        if !self.enabled {
            return;
        }
        self.elapsed += dt;
        self.frames += 1;
        self.min_dt = self.min_dt.min(dt);
        self.max_dt = self.max_dt.max(dt);
        self.sets += u32::from(sent_set);
        self.close_key_probe(status);
        self.record_step(dt, status.rendered_position, velocity);
        self.record_camera(status);
        if let Some(game_forward) = status.game_camera_forward {
            let drift = Self::angle_degrees(view_forward, game_forward);
            self.max_drift_degrees = self.max_drift_degrees.max(drift);
        }
        if self.elapsed >= WINDOW_SECONDS - WINDOW_TOLERANCE {
            self.report = Some(self.build_report());
            self.reset_window();
        }
    }

    // adds every UI frame of the summary to the open window; an empty summary changes nothing
    pub fn record_ui_summary(&mut self, summary: &UiTimingSummary) {
        if !self.enabled || summary.is_empty() {
            return;
        }
        self.ui_cpu_sum += summary.cpu_ms_sum;
        self.ui_cpu_max = self.ui_cpu_max.max(summary.cpu_ms_max);
        self.ui_cpu_samples = self.ui_cpu_samples.saturating_add(summary.cpu_samples);
        self.ui_gpu_sum += summary.gpu_ms_sum;
        self.ui_gpu_max = self.ui_gpu_max.max(summary.gpu_ms_max);
        self.ui_gpu_samples = self.ui_gpu_samples.saturating_add(summary.gpu_samples);
        self.ui_last = summary.last;
    }

    // one frame's marker work: slots live after it, entries in its batch, mod-layer time of the previous batch,
    // reductions of the last build
    pub fn record_markers(&mut self, live_slots: usize, entries_sent: usize, send_us: f32, reductions: u32) {
        if !self.enabled {
            return;
        }
        let entries = u32::try_from(entries_sent).unwrap_or(u32::MAX);
        self.marker_live_slots = u32::try_from(live_slots).unwrap_or(u32::MAX);
        self.marker_entries_sent = self.marker_entries_sent.saturating_add(entries);
        self.marker_max_send_entries = self.marker_max_send_entries.max(entries);
        self.marker_max_send_us = self.marker_max_send_us.max(send_us);
        self.marker_reductions = reductions;
    }

    // one frame's rebuild time in microseconds: the path curve and look-at aim after a path change, and the
    // marker build; 0 for a frame that rebuilt nothing
    pub fn record_path_rebuild(&mut self, us: f32) {
        if !self.enabled || !us.is_finite() {
            return;
        }
        self.path_rebuild_us_max = self.path_rebuild_us_max.max(us);
    }

    // the camera forward vector of the previous and of this playback frame
    pub fn record_aim_turn(&mut self, previous: Vec3, now: Vec3) {
        if !self.enabled {
            return;
        }
        let turn = Self::angle_degrees(previous, now);
        if turn.is_finite() {
            self.aim_turn_max_deg = self.aim_turn_max_deg.max(turn);
        }
    }

    // one look-at ray request, answered or timed out
    pub fn record_look_at_ray(&mut self, answered: bool) {
        if !self.enabled {
            return;
        }
        if answered {
            self.look_at_rays_answered = self.look_at_rays_answered.saturating_add(1);
        } else {
            self.look_at_rays_timed_out = self.look_at_rays_timed_out.saturating_add(1);
        }
    }

    // a flight key went down at `time_seconds`, on the clock of `FrameStatus::time_seconds`; a probe still open
    // keeps its earlier stamp
    pub fn record_key_probe(&mut self, time_seconds: f64) {
        if !self.enabled || !time_seconds.is_finite() {
            return;
        }
        self.key_probe.get_or_insert(time_seconds);
    }

    pub fn take_report(&mut self) -> Option<TelemetryReport> {
        self.report.take()
    }

    // the rendered position is the previous frame's write, so it is judged against the expected steps
    // of the frames from the last rendered position up to the previous frame; the game runs two camera
    // setups per drawn frame and only one of them reports a position
    fn record_step(&mut self, dt: f32, rendered: Option<Vec3>, velocity: Vec3) {
        let speed = velocity.length();
        if rendered.is_none() && self.previous_speed.is_some() {
            self.missed_overrides += 1;
        }
        if let (Some(now), Some(before)) = (rendered, self.previous_rendered)
            && self.steady_since_rendered
            && self.expected_since_rendered > 0.0
        {
            let ratio = (now - before).length() / self.expected_since_rendered;
            self.step_ratio_min = self.step_ratio_min.min(ratio);
            self.step_ratio_max = self.step_ratio_max.max(ratio);
            self.step_samples += 1;
        }
        let steady =
            self.previous_speed.is_some_and(|previous| (speed - previous).abs() < previous * STEADY_SPEED_TOLERANCE);
        let frame_steady = steady && speed >= MIN_STEADY_SPEED && dt > 0.0;
        if rendered.is_some() {
            self.previous_rendered = rendered;
            self.expected_since_rendered = speed * dt;
            self.steady_since_rendered = frame_steady;
        } else {
            self.expected_since_rendered += speed * dt;
            self.steady_since_rendered &= frame_steady;
        }
        self.previous_speed = Some(speed);
    }

    // the first frame whose rendered position differs from the last one closes the open probe
    fn close_key_probe(&mut self, status: &FrameStatus) {
        let (Some(stamp), Some(now)) = (self.key_probe, status.rendered_position) else {
            return;
        };
        if self.previous_rendered == Some(now) {
            return;
        }
        self.key_probe = None;
        let ms = ((status.time_seconds - stamp) * 1000.0) as f32;
        if !ms.is_finite() || ms < 0.0 {
            return;
        }
        self.key_samples[self.key_sample_next] = ms;
        self.key_sample_next = (self.key_sample_next + 1) % Self::KEY_PROBE_CAPACITY;
        self.key_sample_count = (self.key_sample_count + 1).min(Self::KEY_PROBE_CAPACITY);
    }

    // p50, p99 and max of the window's key-to-move samples, all 0 without samples
    fn key_percentiles(&self) -> (f32, f32, f32) {
        if self.key_sample_count == 0 {
            return (0.0, 0.0, 0.0);
        }
        let mut sorted = [0.0_f32; Self::KEY_PROBE_CAPACITY];
        let samples = &mut sorted[..self.key_sample_count];
        samples.copy_from_slice(&self.key_samples[..self.key_sample_count]);
        samples.sort_unstable_by(f32::total_cmp);
        let last = samples.len() - 1;
        let at = |fraction: f32| samples[((last as f32) * fraction).round() as usize];
        (at(0.5), at(0.99), samples[last])
    }

    fn record_camera(&mut self, status: &FrameStatus) {
        if status.game_camera_position.is_some() && status.game_camera_position != self.previous_camera {
            self.camera_updates += 1;
        }
        self.previous_camera = status.game_camera_position;
    }

    fn angle_degrees(a: Vec3, b: Vec3) -> f32 {
        a.normalized().dot(b.normalized()).clamp(-1.0, 1.0).acos().to_degrees()
    }

    fn build_report(&self) -> TelemetryReport {
        let (key_to_move_ms_p50, key_to_move_ms_p99, key_to_move_ms_max) = self.key_percentiles();
        let (step_ratio_min, step_ratio_max) =
            if self.step_samples > 0 { (self.step_ratio_min, self.step_ratio_max) } else { (1.0, 1.0) };
        TelemetryReport {
            fps: self.frames as f32 / self.elapsed,
            min_dt: self.min_dt,
            max_dt: self.max_dt,
            step_ratio_min,
            step_ratio_max,
            aim_turn_max_deg: self.aim_turn_max_deg,
            sets_per_second: self.sets as f32 / self.elapsed,
            camera_updates_per_second: self.camera_updates as f32 / self.elapsed,
            max_drift_degrees: self.max_drift_degrees,
            missed_overrides: self.missed_overrides,
            ui_cpu_ms_avg: Self::average(self.ui_cpu_sum, self.ui_cpu_samples),
            ui_cpu_ms_max: self.ui_cpu_max,
            ui_gpu_ms_avg: Self::average(self.ui_gpu_sum, self.ui_gpu_samples),
            ui_gpu_ms_max: self.ui_gpu_max,
            ui_vertices: self.ui_last.vertices,
            ui_draws: self.ui_last.draws,
            ui_dropped_events: self.ui_last.dropped_events,
            ui_skipped_frames: self.ui_last.skipped_frames,
            marker_live_slots: self.marker_live_slots,
            marker_entries_sent: self.marker_entries_sent,
            marker_max_send_entries: self.marker_max_send_entries,
            marker_max_send_us: self.marker_max_send_us,
            marker_reductions: self.marker_reductions,
            path_rebuild_us_max: self.path_rebuild_us_max,
            look_at_rays_answered: self.look_at_rays_answered,
            look_at_rays_timed_out: self.look_at_rays_timed_out,
            key_to_move_ms_p50,
            key_to_move_ms_p99,
            key_to_move_ms_max,
            key_to_move_samples: u32::try_from(self.key_sample_count).unwrap_or(u32::MAX),
        }
    }

    fn average(sum: f32, samples: u32) -> f32 {
        if samples == 0 { 0.0 } else { sum / samples as f32 }
    }

    fn reset_window(&mut self) {
        self.elapsed = 0.0;
        self.frames = 0;
        self.min_dt = f32::MAX;
        self.max_dt = 0.0;
        self.step_ratio_min = f32::MAX;
        self.step_ratio_max = 0.0;
        self.aim_turn_max_deg = 0.0;
        self.step_samples = 0;
        self.sets = 0;
        self.camera_updates = 0;
        self.max_drift_degrees = 0.0;
        self.missed_overrides = 0;
        self.ui_cpu_sum = 0.0;
        self.ui_cpu_max = 0.0;
        self.ui_cpu_samples = 0;
        self.ui_gpu_sum = 0.0;
        self.ui_gpu_max = 0.0;
        self.ui_gpu_samples = 0;
        self.marker_entries_sent = 0;
        self.marker_max_send_entries = 0;
        self.marker_max_send_us = 0.0;
        self.path_rebuild_us_max = 0.0;
        self.look_at_rays_answered = 0;
        self.look_at_rays_timed_out = 0;
        self.key_sample_count = 0;
        self.key_sample_next = 0;
    }
}
