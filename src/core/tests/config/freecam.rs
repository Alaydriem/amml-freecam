use freecam_core::config::FreecamConfig;

struct Fixture;

impl Fixture {
    // the config as the mod reads it from config.hcl
    fn read(text: &str) -> FreecamConfig {
        hcl::from_str::<FreecamConfig>(text).unwrap().sanitized()
    }
}

#[test]
fn sanitized_fixes_bad_ranges() {
    let mut config = FreecamConfig::default();
    config.motion.min_speed = -1.0;
    config.motion.max_speed = 0.1;
    config.motion.start_speed = 500.0;
    config.motion.pitch_limit = 120.0;
    config.motion.max_frame_dt = 0.0;
    config.exit_wait_frames = 0;

    let config = config.sanitized();

    assert_eq!(config.motion.min_speed, 0.5);
    assert!(config.motion.max_speed >= config.motion.min_speed);
    assert_eq!(config.motion.start_speed, config.motion.max_speed);
    assert_eq!(config.motion.pitch_limit, 89.9);
    assert_eq!(config.motion.max_frame_dt, 0.1);
    assert_eq!(config.exit_wait_frames, 4);
}

#[test]
fn tab_as_bar_key_falls_back_to_ctrl_e() {
    assert_eq!(Fixture::read(r#"keys { toggle_bar = "Tab" }"#).keys.toggle_bar, "F3+E");
}

#[test]
fn marker_and_look_at_values_are_clamped() {
    let low = Fixture::read(
        r#"
markers {
  enabled       = false
  max_shapes    = 1
  max_per_send  = 1
  line_weight   = 0
  frustum_depth = 0.0
}
look_at {
  enabled      = false
  max_distance = 1.0
}
"#,
    );
    assert!(!low.markers.enabled);
    assert_eq!(low.markers.max_shapes, 2560);
    assert_eq!(low.markers.max_per_send, 50);
    assert_eq!(low.markers.line_weight, 1);
    assert_eq!(low.markers.frustum_depth, 0.3);
    assert!(!low.look_at.enabled);
    assert_eq!(low.look_at.max_distance, 8.0);

    let high = Fixture::read(
        r#"
markers {
  max_shapes    = 99999
  max_per_send  = 5000
  line_weight   = 7
  frustum_depth = 40.0
}
look_at {
  max_distance = 9000.0
}
"#,
    );
    assert_eq!(high.markers.max_shapes, 8192);
    assert_eq!(high.markers.max_per_send, 800);
    assert_eq!(high.markers.line_weight, 2);
    assert_eq!(high.markers.frustum_depth, 5.0);
    assert_eq!(high.look_at.max_distance, 512.0);
}

#[test]
fn non_finite_marker_and_look_at_floats_use_defaults() {
    let mut config = FreecamConfig::default();
    config.markers.frustum_depth = f32::NAN;
    config.look_at.max_distance = f32::INFINITY;

    let config = config.sanitized();

    assert_eq!(config.markers.frustum_depth, 1.2);
    assert_eq!(config.look_at.max_distance, 256.0);
}

#[test]
fn screenshot_folder_is_read_and_a_blank_one_falls_back() {
    assert_eq!(Fixture::read(r#"screenshot { folder = "D:\\shots" }"#).screenshot.folder, r"D:\shots");
    assert_eq!(
        Fixture::read(r#"screenshot { folder = "  " }"#).screenshot.folder,
        r"%USERPROFILE%\Pictures\Screenshots"
    );
}

#[test]
fn exit_wait_and_telemetry_are_top_level_settings() {
    let config = Fixture::read("exit_wait_frames = 2\ntelemetry = true\n");

    assert_eq!(config.exit_wait_frames, 4);
    assert!(config.telemetry);
}
