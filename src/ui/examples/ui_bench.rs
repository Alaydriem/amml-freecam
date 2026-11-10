//! Headless UI bench: runs the freecam views for a fixed number of frames with a fixed snapshot, as the mod manager
//! runs the Overlay layer, and prints the cost of the pass (freecam's work) and of the tessellation (the manager's).
//!
//! `mise run bench-ui`. A dev tool, never loaded by the game, so it prints to stdout.

use std::time::{Duration, Instant};

use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::{Context, Pos2, RawInput, Rect, vec2};
use freecam_core::config::KeyConfig;
use freecam_core::structs::{UiAction, UiLegend, UiSnapshot};
use freecam_ui::runtime::FreecamUi;

struct Bench;

impl Bench {
    const FRAMES: usize = 600;

    const SCENES: [usize; 2] = [5, 256];
    const POINT_SPACING: f32 = 4.0;

    fn run() {
        for points in Self::SCENES {
            Self::scene(points);
        }
    }

    // bar mode with `points` points evenly spread, the last one selected
    fn snapshot(points: usize) -> UiSnapshot {
        let mut snapshot = UiSnapshot {
            visible: true,
            cursor_mode: true,
            can_screenshot: true,
            point_count: points as u32,
            length: (points - 1) as f32 * Self::POINT_SPACING,
            playhead: 2.0,
            playhead_seconds: 0.5,
            total_seconds: 4.5,
            speed: 6.0,
            selection: Some(points - 1),
            ..UiSnapshot::default()
        };
        for (index, distance) in snapshot.point_distances[..points].iter_mut().enumerate() {
            *distance = index as f32 * Self::POINT_SPACING;
        }
        snapshot
    }

    fn input(time: f64) -> RawInput {
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0))),
            time: Some(time),
            ..Default::default()
        };
        input.viewports.entry(input.viewport_id).or_default().native_pixels_per_point = Some(1.0);
        input
    }

    fn scene(points: usize) {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::Embedded);
        let mut ui = FreecamUi::new();
        let snapshot = Self::snapshot(points);
        let legend = UiLegend::new(&KeyConfig::default());
        let mut actions: Vec<UiAction> = Vec::with_capacity(16);

        let mut run = Vec::with_capacity(Self::FRAMES);
        let mut tessellate = Vec::with_capacity(Self::FRAMES);
        let mut first = Duration::ZERO;
        let mut vertices = 0;
        for frame in 0..Self::FRAMES {
            let started = Instant::now();
            let mut output = ctx.run_pass(Self::input(frame as f64 / 60.0), |ctx| {
                actions.clear();
                ui.show(ctx, &snapshot, &legend, &mut actions);
            });
            let ran = Instant::now();
            output.textures_delta.clear();
            let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
            let tessellated = Instant::now();
            if frame == 0 {
                first = tessellated - started;
                continue;
            }
            vertices = primitives
                .iter()
                .map(|primitive| match &primitive.primitive {
                    egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
                    egui::epaint::Primitive::Callback(_) => 0,
                })
                .sum();
            run.push(ran - started);
            tessellate.push(tessellated - ran);
        }

        println!("ui_bench: {} frames at 1920x1080, strip + legend + bar with {points} points", Self::FRAMES);
        println!("first frame (fonts, atlas): {:.3} ms", Self::ms(first));
        println!("last frame: {vertices} vertices");
        println!("phase                    avg ms    p95 ms   (frames 2..{})", Self::FRAMES);
        Self::row("run (freecam)", &mut run);
        Self::row("tessellate (manager)", &mut tessellate);
    }

    fn row(name: &str, samples: &mut [Duration]) {
        samples.sort_unstable();
        let average = samples.iter().sum::<Duration>() / samples.len().max(1) as u32;
        let p95 = samples.get(samples.len() * 95 / 100).copied().unwrap_or_default();
        println!("{name:<22} {:>8.4}  {:>8.4}", Self::ms(average), Self::ms(p95));
    }

    fn ms(duration: Duration) -> f64 {
        duration.as_secs_f64() * 1000.0
    }
}

fn main() {
    Bench::run();
}
