use amml_ui::font::FontSource;
use amml_ui::text::UiLocale;
use amml_ui::theme::Theme;
use egui::{Context, Event, PointerButton, Pos2, RawInput, Rect, Shape, vec2};
use freecam_core::config::KeyConfig;
use freecam_core::structs::{UiAction, UiLegend, UiSnapshot};
use freecam_ui::runtime::{FreecamLocale, FreecamUi};

/// A pointer event as the mod manager feeds it to egui, in points.
#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    PointerMoved { x: f32, y: f32 },
    PointerButton { x: f32, y: f32, button: u8, pressed: bool },
}

/// One vertex of the tessellated frame, in points, with its premultiplied colour.
#[derive(Debug, Clone, Copy)]
pub struct TestVertex {
    pub x: f32,
    pub y: f32,
    pub rgba: [u8; 4],
}

/// What one frame drew and did.
#[derive(Debug, Default)]
pub struct FrameResult {
    pub actions: Vec<UiAction>,
    pub vertices: Vec<TestVertex>,
    pub rect: Option<Rect>,
    pub wants_pointer: bool,
}

/// The freecam views in a amml-ui context as the mod manager makes it, run on a 1920x1080 screen.
pub struct UiFixture {
    pub ui: FreecamUi,
    pub ctx: Context,
    pub snapshot: UiSnapshot,
    pub legend: UiLegend,
    pub pixels_per_point: f32,
    pub time: f64,
    actions: Vec<UiAction>,
    last: FrameResult,
}

impl UiFixture {
    pub const SETTLE_FRAMES: usize = 12;
    pub const DISTANCES: [f32; 3] = [0.0, 10.0, 40.0];
    pub const SCREEN: [f32; 2] = [1920.0, 1080.0];

    pub fn new() -> Self {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::Embedded);
        Self {
            ui: FreecamUi::new(),
            ctx,
            snapshot: UiSnapshot {
                visible: true,
                can_screenshot: true,
                point_count: 3,
                total_seconds: 4.5,
                ..Default::default()
            },
            legend: UiLegend::new(&KeyConfig::default()),
            pixels_per_point: 1.0,
            time: 0.0,
            actions: Vec::with_capacity(16),
            last: FrameResult::default(),
        }
    }

    pub fn cursor_mode(mut self) -> Self {
        self.snapshot.cursor_mode = true;
        self
    }

    /// Bar mode with three points at 0, 10 and 40 blocks on a 40 block path.
    pub fn with_path(mut self) -> Self {
        self.snapshot.cursor_mode = true;
        self.snapshot.point_count = 3;
        self.snapshot.point_distances[..3].copy_from_slice(&Self::DISTANCES);
        self.snapshot.length = 40.0;
        self.snapshot.speed = 6.0;
        self
    }

    /// Bar mode with `points` points one block apart.
    pub fn with_points(mut self, points: usize) -> Self {
        self.snapshot.cursor_mode = true;
        self.snapshot.point_count = points as u32;
        for (index, distance) in self.snapshot.point_distances[..points].iter_mut().enumerate() {
            *distance = index as f32;
        }
        self.snapshot.length = points.saturating_sub(1) as f32;
        self.snapshot.speed = 6.0;
        self
    }

    /// A new context with the theme, as the mod manager makes one after the renderer lost its textures.
    pub fn new_context(&mut self) {
        self.ctx = Context::default();
        Theme::install(&self.ctx, FontSource::Embedded);
    }

    /// The views' words in the language of `code`, as the mod manager sets them: empty for English, `en_XA` for the
    /// pseudo-locale.
    pub fn locale(self, code: &str) -> Self {
        UiLocale::set(&self.ctx, FreecamLocale::localizer(code));
        self
    }

    /// Runs one frame and returns each painted text with its rect on the screen.
    pub fn texts(&mut self) -> Vec<(String, Rect)> {
        let (output, _) = self.pass(&[]);
        let mut texts = Vec::new();
        for clipped in output.shapes {
            Self::collect_texts(&clipped.shape, &mut texts);
        }
        texts
    }

    fn collect_texts(shape: &Shape, texts: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(text) => {
                texts.push((text.galley.text().to_owned(), text.galley.rect.translate(text.pos.to_vec2())));
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| Self::collect_texts(shape, texts)),
            _ => {}
        }
    }

    /// Runs enough empty frames for every hover and selection transition to finish.
    pub fn settle(&mut self) {
        for _ in 0..Self::SETTLE_FRAMES {
            self.frame(&[]);
        }
    }

    /// Runs one frame a sixtieth of a second after the last and tessellates it.
    pub fn frame(&mut self, events: &[UiEvent]) -> &FrameResult {
        let (output, rect) = self.pass(events);
        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let mut vertices = Vec::new();
        for primitive in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                vertices.extend(mesh.vertices.iter().map(|v| TestVertex {
                    x: v.pos.x,
                    y: v.pos.y,
                    rgba: v.color.to_array(),
                }));
            }
        }
        self.last = FrameResult {
            actions: self.actions.clone(),
            vertices,
            rect: rect.is_positive().then_some(rect),
            wants_pointer: self.ui.pointer_over_panel(),
        };
        &self.last
    }

    /// Runs one frame of the views only, as the mod manager calls the frame handler, with nothing tessellated.
    pub fn run(&mut self) {
        let _ = self.pass(&[]);
    }

    fn pass(&mut self, events: &[UiEvent]) -> (egui::FullOutput, Rect) {
        self.time += 1.0 / 60.0;
        self.actions.clear();
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(Self::SCREEN[0], Self::SCREEN[1]))),
            time: Some(self.time),
            events: events.iter().map(|event| Self::egui_event(*event)).collect(),
            ..Default::default()
        };
        input.viewports.entry(input.viewport_id).or_default().native_pixels_per_point = Some(self.pixels_per_point);
        let mut rect = Rect::NOTHING;
        let ui = &mut self.ui;
        let snapshot = &self.snapshot;
        let legend = &self.legend;
        let actions = &mut self.actions;
        let mut output = self.ctx.run_pass(input, |ctx| rect = ui.show(ctx, snapshot, legend, actions));
        output.textures_delta.clear();
        (output, rect)
    }

    fn egui_event(event: UiEvent) -> Event {
        match event {
            UiEvent::PointerMoved { x, y } => Event::PointerMoved(Pos2::new(x, y)),
            UiEvent::PointerButton { x, y, button, pressed } => Event::PointerButton {
                pos: Pos2::new(x, y),
                button: if button == 1 { PointerButton::Secondary } else { PointerButton::Primary },
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        }
    }
}
