use freecam_core::services::SmoothnessTelemetry;
use freecam_core::structs::{FrameStatus, TelemetryReport, UiTiming, UiTimingSummary, Vec3};

use crate::support::frames::Frames;

const DT: f32 = 1.0 / 120.0;

struct Fixture;

impl Fixture {
    // hands telemetry one UI frame, as a summary of one
    fn record_ui(telemetry: &mut SmoothnessTelemetry, timing: UiTiming) {
        let mut summary = UiTimingSummary::new();
        summary.add(timing);
        telemetry.record_ui_summary(&summary);
    }

    fn status(z: f32, game_forward: Option<Vec3>) -> FrameStatus {
        FrameStatus {
            rendered_position: Some(Vec3::new(0.0, 0.0, z)),
            game_camera_forward: game_forward,
            ..Frames::status(0.0)
        }
    }

    // feeds frames and returns the last report produced
    fn run(
        telemetry: &mut SmoothnessTelemetry,
        frames: usize,
        step_for: impl Fn(usize) -> f32,
        game_forward: Option<Vec3>,
    ) -> Option<TelemetryReport> {
        let view = Vec3::new(0.0, 0.0, 1.0);
        let mut z = 0.0;
        let mut report = None;
        for frame in 0..frames {
            z += step_for(frame);
            telemetry.record(DT, &Self::status(z, game_forward), true, Vec3::new(0.0, 0.0, 6.0), view);
            if let Some(ready) = telemetry.take_report() {
                report = Some(ready);
            }
        }
        report
    }
}

#[test]
fn even_steps_give_ratio_one() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let report = Fixture::run(&mut telemetry, 240, |_| 6.0 * DT, None).unwrap();
    assert!((report.step_ratio_min - 1.0).abs() < 0.01);
    assert!((report.step_ratio_max - 1.0).abs() < 0.01);
    assert!((report.fps - 120.0).abs() < 1.0);
    assert!((report.sets_per_second - 120.0).abs() < 1.0);
}

#[test]
fn every_other_frame_steps_are_caught() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let report = Fixture::run(&mut telemetry, 240, |f| if f % 2 == 0 { 12.0 * DT } else { 0.0 }, None).unwrap();
    assert!(report.step_ratio_max >= 1.9);
    assert!(report.step_ratio_min <= 0.1);
}

#[test]
fn uneven_frame_times_with_even_steps_give_ratio_one() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut rendered_z = 0.0;
    let mut written_z = 0.0;
    let mut report = None;
    for frame in 0..400 {
        let dt = if frame % 2 == 0 { 0.007 } else { 0.016 };
        // the shim reports the pose written on the previous frame
        rendered_z = written_z;
        written_z += 6.0 * dt;
        telemetry.record(dt, &Fixture::status(rendered_z, None), true, Vec3::new(0.0, 0.0, 6.0), view);
        if let Some(ready) = telemetry.take_report() {
            report = Some(ready);
        }
    }
    let report = report.unwrap();
    assert!((report.step_ratio_min - 1.0).abs() < 0.01);
    assert!((report.step_ratio_max - 1.0).abs() < 0.01);
    assert!(rendered_z > 0.0);
}

#[test]
fn missing_rendered_position_counts_as_missed_override() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut report = None;
    for frame in 0..240 {
        let status = if frame == 100 { Frames::status(0.0) } else { Fixture::status(frame as f32, None) };
        telemetry.record(DT, &status, true, Vec3::new(0.0, 0.0, 6.0), view);
        if let Some(ready) = telemetry.take_report() {
            report = Some(ready);
        }
    }
    assert_eq!(report.unwrap().missed_overrides, 1);
}

#[test]
fn drift_is_measured() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let radians = 10.0_f32.to_radians();
    let game = Vec3::new(radians.sin(), 0.0, radians.cos());
    let report = Fixture::run(&mut telemetry, 240, |_| 6.0 * DT, Some(game)).unwrap();
    assert!((report.max_drift_degrees - 10.0).abs() < 0.1);
}

#[test]
fn camera_updates_count_position_changes() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let mut report = None;
    for frame in 0..240 {
        let status =
            FrameStatus { game_camera_position: Some(Vec3::new((frame / 2) as f32, 0.0, 0.0)), ..Frames::status(0.0) };
        telemetry.record(DT, &status, true, Vec3::default(), Vec3::new(0.0, 0.0, 1.0));
        if let Some(ready) = telemetry.take_report() {
            report = Some(ready);
        }
    }
    let report = report.unwrap();
    assert!((report.camera_updates_per_second - 60.0).abs() < 1.0);
}

#[test]
fn no_report_before_two_seconds() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    assert!(Fixture::run(&mut telemetry, 228, |_| 6.0 * DT, None).is_none());
}

#[test]
fn disabled_reports_nothing() {
    let mut telemetry = SmoothnessTelemetry::new(false);
    assert!(Fixture::run(&mut telemetry, 360, |_| 6.0 * DT, None).is_none());
}

#[test]
fn window_resets_after_a_report() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    assert!(Fixture::run(&mut telemetry, 240, |_| 6.0 * DT, None).is_some());
    assert!(telemetry.take_report().is_none());
    assert!(Fixture::run(&mut telemetry, 120, |_| 6.0 * DT, None).is_none());
}

#[test]
fn ui_timing_reaches_the_report() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let first = UiTiming { cpu_ms: 0.4, gpu_ms: Some(1.0), vertices: 100, draws: 2, ..UiTiming::default() };
    let second =
        UiTiming { cpu_ms: 0.8, gpu_ms: Some(3.0), vertices: 300, draws: 4, dropped_events: 5, skipped_frames: 6 };
    Fixture::record_ui(&mut telemetry, first);
    Fixture::record_ui(&mut telemetry, second);

    let report = Fixture::run(&mut telemetry, 240, |_| 6.0 * DT, None).unwrap();

    assert!((report.ui_cpu_ms_avg - 0.6).abs() < 1e-5);
    assert!((report.ui_cpu_ms_max - 0.8).abs() < 1e-5);
    assert!((report.ui_gpu_ms_avg - 2.0).abs() < 1e-5);
    assert!((report.ui_gpu_ms_max - 3.0).abs() < 1e-5);
    assert_eq!((report.ui_vertices, report.ui_draws), (300, 4));
    assert_eq!((report.ui_dropped_events, report.ui_skipped_frames), (5, 6));
}

#[test]
fn ui_report_is_zero_without_samples() {
    let mut telemetry = SmoothnessTelemetry::new(true);

    let report = Fixture::run(&mut telemetry, 240, |_| 6.0 * DT, None).unwrap();

    assert_eq!(report.ui_cpu_ms_avg, 0.0);
    assert_eq!(report.ui_gpu_ms_avg, 0.0);
    assert_eq!(report.ui_gpu_ms_max, 0.0);
}

#[test]
fn ui_timing_ignored_when_disabled() {
    let mut telemetry = SmoothnessTelemetry::new(false);
    Fixture::record_ui(&mut telemetry, UiTiming { cpu_ms: 0.4, ..UiTiming::default() });

    assert!(Fixture::run(&mut telemetry, 360, |_| 6.0 * DT, None).is_none());
}

#[test]
fn ui_summary_of_several_frames_reaches_the_report() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let mut summary = UiTimingSummary::new();
    summary.add(UiTiming { cpu_ms: 0.4, gpu_ms: Some(1.0), vertices: 100, draws: 2, ..UiTiming::default() });
    summary.add(UiTiming { cpu_ms: 0.9, gpu_ms: None, vertices: 200, draws: 3, ..UiTiming::default() });
    summary.add(UiTiming {
        cpu_ms: 0.5,
        gpu_ms: Some(3.0),
        vertices: 300,
        draws: 4,
        dropped_events: 5,
        skipped_frames: 6,
    });
    telemetry.record_ui_summary(&summary);
    telemetry.record_ui_summary(&UiTimingSummary::new());

    let report = Fixture::run(&mut telemetry, 240, |_| 6.0 * DT, None).unwrap();

    assert!((report.ui_cpu_ms_avg - 0.6).abs() < 1e-5);
    assert!((report.ui_cpu_ms_max - 0.9).abs() < 1e-5);
    assert!((report.ui_gpu_ms_avg - 2.0).abs() < 1e-5);
    assert!((report.ui_gpu_ms_max - 3.0).abs() < 1e-5);
    assert_eq!((report.ui_vertices, report.ui_draws), (300, 4));
    assert_eq!((report.ui_dropped_events, report.ui_skipped_frames), (5, 6));
}

#[test]
fn marker_counters_reach_the_report() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut report = None;
    for frame in 0..240 {
        let (entries, send_us) = match frame {
            0 => (400, 80.0),
            1 => (250, 120.0),
            _ => (0, 0.0),
        };
        telemetry.record_markers(650, entries, send_us, 2);
        telemetry.record(DT, &Fixture::status(0.0, None), true, Vec3::default(), view);
        report = report.or(telemetry.take_report());
    }
    let report = report.unwrap();
    assert_eq!(report.marker_live_slots, 650);
    assert_eq!(report.marker_entries_sent, 650);
    assert_eq!(report.marker_max_send_entries, 400);
    assert!((report.marker_max_send_us - 120.0).abs() < 1e-3);
    assert_eq!(report.marker_reductions, 2);
}

#[test]
fn marker_counters_reset_with_the_window() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut reports = Vec::new();
    for frame in 0..480 {
        let entries = if frame == 0 { 400 } else { 0 };
        telemetry.record_markers(10, entries, 0.0, 0);
        telemetry.record(DT, &Fixture::status(0.0, None), true, Vec3::default(), view);
        reports.extend(telemetry.take_report());
    }
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].marker_entries_sent, 400);
    assert_eq!(reports[1].marker_entries_sent, 0);
    assert_eq!(reports[1].marker_max_send_entries, 0);
    assert_eq!(reports[1].marker_live_slots, 10);
}

#[test]
fn look_at_ray_counters_reach_the_report_and_reset() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut reports = Vec::new();
    for frame in 0..480 {
        match frame {
            0 | 5 => telemetry.record_look_at_ray(true),
            9 => telemetry.record_look_at_ray(false),
            _ => {}
        }
        telemetry.record(DT, &Fixture::status(0.0, None), true, Vec3::default(), view);
        reports.extend(telemetry.take_report());
    }
    assert_eq!(reports.len(), 2);
    assert_eq!((reports[0].look_at_rays_answered, reports[0].look_at_rays_timed_out), (2, 1));
    assert_eq!((reports[1].look_at_rays_answered, reports[1].look_at_rays_timed_out), (0, 0));
}

#[test]
fn aim_turn_max_reaches_the_report_and_resets() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut reports = Vec::new();
    for frame in 0..480 {
        match frame {
            3 => telemetry.record_aim_turn(view, Vec3::new(1.0, 0.0, 1.0)),
            7 => telemetry.record_aim_turn(view, Vec3::new(0.0, 0.1, 1.0)),
            _ => telemetry.record_aim_turn(view, view),
        }
        telemetry.record(DT, &Fixture::status(0.0, None), true, Vec3::default(), view);
        reports.extend(telemetry.take_report());
    }
    assert_eq!(reports.len(), 2);
    assert!((reports[0].aim_turn_max_deg - 45.0).abs() < 1e-3, "{}", reports[0].aim_turn_max_deg);
    assert_eq!(reports[1].aim_turn_max_deg, 0.0);
}

// the live frame pattern: two `fc_frame` calls per drawn frame, 0.6 ms and 16.1 ms apart, and only
// the second has a rendered position; the camera writes `factor(frame)` times the expected step
fn alternating_report(factor: impl Fn(usize) -> f32) -> TelemetryReport {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut written_z = 0.0;
    let mut report = None;
    for frame in 0..600 {
        let dt = if frame % 2 == 0 { 0.0006 } else { 0.0161 };
        let rendered = if frame % 2 == 1 { Some(Vec3::new(0.0, 0.0, written_z)) } else { None };
        written_z += 6.0 * dt * factor(frame);
        let status = FrameStatus { rendered_position: rendered, ..Frames::status(0.0) };
        telemetry.record(dt, &status, true, Vec3::new(0.0, 0.0, 6.0), view);
        if let Some(ready) = telemetry.take_report() {
            report = Some(ready);
        }
    }
    report.unwrap()
}

#[test]
fn frames_without_a_rendered_position_still_give_step_samples() {
    let even = alternating_report(|_| 1.0);
    assert!((even.step_ratio_min - 1.0).abs() < 0.01, "{}", even.step_ratio_min);
    assert!((even.step_ratio_max - 1.0).abs() < 0.01, "{}", even.step_ratio_max);

    let uneven = alternating_report(|frame| if (frame / 2) % 2 == 0 { 1.5 } else { 0.5 });
    assert!(uneven.step_ratio_max > 1.3, "{}", uneven.step_ratio_max);
    assert!(uneven.step_ratio_min < 0.7, "{}", uneven.step_ratio_min);
}

// short frames, so many probes fit in one window
const PROBE_DT: f32 = 0.001;

// one frame at `time_seconds` with the camera rendered at `z`
fn probe_frame(telemetry: &mut SmoothnessTelemetry, time_seconds: f64, z: f32) -> Option<TelemetryReport> {
    let status = FrameStatus { rendered_position: Some(Vec3::new(0.0, 0.0, z)), ..Frames::status(time_seconds) };
    telemetry.record(PROBE_DT, &status, true, Vec3::default(), Vec3::new(0.0, 0.0, 1.0));
    telemetry.take_report()
}

// runs frames that do not move the camera until the window closes and returns its report
fn close_window(telemetry: &mut SmoothnessTelemetry, time_seconds: f64, z: f32) -> TelemetryReport {
    (0..2100).find_map(|frame| probe_frame(telemetry, time_seconds + frame as f64 * 0.001, z)).unwrap()
}

#[test]
fn key_probe_measures_time_to_first_pose_change() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    probe_frame(&mut telemetry, 0.996, 0.0);
    telemetry.record_key_probe(1.000);
    probe_frame(&mut telemetry, 1.004, 0.0);
    probe_frame(&mut telemetry, 1.008, 0.0);
    probe_frame(&mut telemetry, 1.012, 0.5);

    let report = close_window(&mut telemetry, 1.016, 0.5);

    assert_eq!(report.key_to_move_samples, 1);
    assert!((report.key_to_move_ms_max - 12.0).abs() < 0.01, "{}", report.key_to_move_ms_max);
    assert!((report.key_to_move_ms_p50 - 12.0).abs() < 0.01, "{}", report.key_to_move_ms_p50);
}

#[test]
fn second_probe_before_move_keeps_the_first_stamp() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    probe_frame(&mut telemetry, 0.996, 0.0);
    telemetry.record_key_probe(1.000);
    telemetry.record_key_probe(1.005);
    probe_frame(&mut telemetry, 1.010, 0.5);

    let report = close_window(&mut telemetry, 1.014, 0.5);

    assert_eq!(report.key_to_move_samples, 1);
    assert!((report.key_to_move_ms_max - 10.0).abs() < 0.01, "{}", report.key_to_move_ms_max);
}

#[test]
fn key_probe_samples_are_bounded() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let mut time = 0.0;
    let mut z = 0.0;
    probe_frame(&mut telemetry, time, z);
    // the first 172 moves take 50 ms, the last 128 take 4 ms, all inside one window
    for pair in 0..300 {
        let latency = if pair < 172 { 0.050 } else { 0.004 };
        telemetry.record_key_probe(time);
        time += latency;
        z += 1.0;
        assert!(probe_frame(&mut telemetry, time, z).is_none());
    }

    let report = close_window(&mut telemetry, time + 0.001, z);

    assert_eq!(report.key_to_move_samples as usize, SmoothnessTelemetry::KEY_PROBE_CAPACITY);
    assert!((report.key_to_move_ms_max - 4.0).abs() < 0.01, "{}", report.key_to_move_ms_max);
}

#[test]
fn key_probe_report_is_zero_without_samples() {
    let mut telemetry = SmoothnessTelemetry::new(true);

    let report = close_window(&mut telemetry, 0.0, 0.0);

    assert_eq!(report.key_to_move_samples, 0);
    assert_eq!(report.key_to_move_ms_max, 0.0);
}

#[test]
fn path_rebuild_time_is_the_window_max_and_resets() {
    let mut telemetry = SmoothnessTelemetry::new(true);
    let view = Vec3::new(0.0, 0.0, 1.0);
    let mut reports = Vec::new();
    for frame in 0..480 {
        match frame {
            0 => telemetry.record_path_rebuild(300.0),
            1 => telemetry.record_path_rebuild(900.0),
            2 => telemetry.record_path_rebuild(40.0),
            _ => telemetry.record_path_rebuild(0.0),
        }
        telemetry.record(DT, &Fixture::status(0.0, None), true, Vec3::default(), view);
        reports.extend(telemetry.take_report());
    }
    assert_eq!(reports.len(), 2);
    assert!((reports[0].path_rebuild_us_max - 900.0).abs() < 1e-3);
    assert_eq!(reports[1].path_rebuild_us_max, 0.0);
}
