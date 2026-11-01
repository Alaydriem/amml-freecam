use std::sync::{Arc, Mutex};

use alaydriem_freecam::services::{FreecamFrameHandler, RenderClock, UiHandoff};
use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::frame::{FrameInputData, LayerKind};
use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use freecam_core::config::{FreecamConfig, LookAtConfig};
use freecam_core::services::FreecamController;
use freecam_core::structs::{CameraPose, FrameStatus, FreecamAction, StartLeases, Vec3};
use freecam_ui::views::BarLayout;

struct Fixture {
    handoff: Arc<Mutex<UiHandoff>>,
    clock: Arc<RenderClock>,
    handler: FreecamFrameHandler,
    ctx: Context,
}

impl Fixture {
    fn new() -> Self {
        let handoff = Arc::new(Mutex::new(UiHandoff::new()));
        let clock = Arc::new(RenderClock::new());
        let handler = FreecamFrameHandler::new(Arc::clone(&handoff), Arc::clone(&clock));
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::Embedded);
        Self { handoff, clock, handler, ctx }
    }

    fn status(time_seconds: f64) -> FrameStatus {
        FrameStatus {
            time_seconds,
            in_world: true,
            menu_open: false,
            focused: true,
            ready: true,
            player_camera_active: true,
            start_pose: CameraPose::new(Vec3::new(10.0, 64.0, -5.0), 90.0, 10.0),
            health: 20,
            dimension: 0,
            rendered_position: None,
            game_camera_forward: None,
            game_camera_position: None,
            fov_y_degrees: 70.0,
            aspect: 16.0 / 9.0,
            last_shape_send_us: 0.0,
            handler_generation: 1,
            present_count: 0,
            camera_held_by_other: false,
        }
    }

    fn press(controller: &mut FreecamController, action: FreecamAction) {
        controller.on_link_event(&ModEvent::Keybind { action: action.index(), phase: KeybindPhase::Pressed });
        if controller.take_start_request() {
            controller.finish_start(StartLeases::Granted);
        }
    }

    // a controller in fly mode, or in bar mode, published into the hand-off
    fn publish(&self, bar: bool) -> FreecamController {
        // look-at off, so Add point stores a key at once instead of asking for a ray
        let config = FreecamConfig {
            look_at: LookAtConfig { enabled: false, ..LookAtConfig::default() },
            ..FreecamConfig::default()
        };
        let mut controller = FreecamController::new(config);
        Self::press(&mut controller, FreecamAction::Toggle);
        controller.frame(&Self::status(0.05));
        if bar {
            Self::press(&mut controller, FreecamAction::ToggleBar);
            controller.frame(&Self::status(0.10));
        }
        self.handoff.lock().unwrap().publish(&mut controller);
        controller
    }

    fn run(&mut self, layer: LayerKind) -> FrameReply {
        self.run_with(layer, Vec::new())
    }

    fn run_with(&mut self, layer: LayerKind, events: Vec<Event>) -> FrameReply {
        let data = FrameInputData {
            width: 1920,
            height: 1080,
            pixels_per_point: 1.0,
            time_secs: 0.0,
            events: &[],
            allowed_layers: FrameInputData::LAYER_HUD | FrameInputData::LAYER_OVERLAY,
            input_flags: 0,
        };
        let input = FrameInputView::new(data, layer);
        let mut reply = None;
        let raw = RawInput { screen_rect: Some(Self::screen()), events, ..Default::default() };
        let handler = &mut self.handler;
        let mut output = self.ctx.run_pass(raw, |ctx| reply = Some(handler.frame(ctx, &input)));
        output.textures_delta.clear();
        reply.unwrap()
    }

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0))
    }
}

#[test]
fn hud_layer_draws_nothing() {
    let mut fixture = Fixture::new();
    fixture.publish(false);

    let reply = fixture.run(LayerKind::Hud);

    assert!(!reply.layer_rect.is_positive());
    assert_eq!(fixture.clock.frames(), 0);
}

#[test]
fn overlay_layer_draws_the_views_of_the_published_snapshot() {
    let mut fixture = Fixture::new();
    let idle = fixture.run(LayerKind::Overlay);
    fixture.publish(false);

    let flying = fixture.run(LayerKind::Overlay);

    assert!(!idle.layer_rect.is_positive());
    assert!(flying.layer_rect.is_positive());
}

#[test]
fn overlay_layer_updates_render_clock() {
    let mut fixture = Fixture::new();

    fixture.run(LayerKind::Overlay);
    fixture.run(LayerKind::Overlay);

    assert_eq!(fixture.clock.frames(), 2);
    assert_eq!(fixture.clock.aspect(), Some(1920.0 / 1080.0));
}

#[test]
fn overlay_layer_publishes_actions_to_the_handoff() {
    let mut fixture = Fixture::new();
    let mut controller = fixture.publish(true);
    let add = BarLayout::new(Fixture::screen()).add.center();
    fixture.run(LayerKind::Overlay);
    fixture.run(LayerKind::Overlay);

    let button = |pressed| Event::PointerButton {
        pos: add,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    fixture.run_with(LayerKind::Overlay, vec![Event::PointerMoved(add), button(true), button(false)]);
    fixture.handoff.lock().unwrap().apply_actions(&mut controller);

    assert_eq!(controller.path().keys().len(), 1);
}
