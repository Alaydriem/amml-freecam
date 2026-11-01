use std::sync::{Arc, Mutex, TryLockError};
use std::time::Instant;

use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_core::structs::frame::{CursorShape, LayerKind};
use egui::{Context, CursorIcon, Rect};
use freecam_core::config::KeyConfig;
use freecam_core::structs::{UiAction, UiLegend, UiSnapshot, UiTiming};
use freecam_ui::runtime::FreecamUi;

use crate::services::{RenderClock, UiHandoff};

// freecam's egui pass, called by the mod manager on the render thread once per drawn layer. The Hud layer only hides
// the vanilla HUD and draws nothing; the Overlay layer draws the strip, the legend, the playback bar and the toast.
//
// Bound: `actions` holds at most UiHandoff::ACTION_CAPACITY clicks of one frame, reserved in `new()`
pub struct FreecamFrameHandler {
    handoff: Arc<Mutex<UiHandoff>>,
    clock: Arc<RenderClock>,
    ui: FreecamUi,
    snapshot: UiSnapshot,
    legend: UiLegend,
    legend_revision: u64,
    actions: Vec<UiAction>,
}

impl FreecamFrameHandler {
    pub fn new(handoff: Arc<Mutex<UiHandoff>>, clock: Arc<RenderClock>) -> Self {
        Self {
            handoff,
            clock,
            ui: FreecamUi::new(),
            snapshot: UiSnapshot::default(),
            legend: UiLegend::new(&KeyConfig::default()),
            legend_revision: 0,
            actions: Vec::with_capacity(UiHandoff::ACTION_CAPACITY),
        }
    }

    fn overlay(&mut self, ctx: &Context, input: &FrameInputView) -> FrameReply {
        let started = self.clock.measure().then(Instant::now);
        self.take_snapshot();
        self.actions.clear();
        let rect = self.ui.show(ctx, &self.snapshot, &self.legend, &mut self.actions);
        self.actions.truncate(UiHandoff::ACTION_CAPACITY);
        let data = input.data();
        self.clock.record_frame(data.width, data.height, self.ui.pointer_over_panel());
        let cpu_ms = started.map_or(0.0, |started| started.elapsed().as_secs_f32() * 1000.0);
        if let Ok(mut handoff) = self.handoff.lock() {
            handoff.push_frame(&self.actions, UiTiming { cpu_ms, ..UiTiming::default() });
        }
        if !rect.is_positive() {
            return FrameReply { cursor: CursorShape::Default, layer_rect: Rect::NOTHING };
        }
        let hand = ctx.output(|output| output.cursor_icon == CursorIcon::PointingHand);
        let cursor = if hand { CursorShape::Hand } else { CursorShape::Arrow };
        FrameReply { cursor, layer_rect: rect }
    }

    // the newest snapshot, and the legend when it changed; a busy hand-off keeps the last ones for this frame
    fn take_snapshot(&mut self) {
        let handoff = match self.handoff.try_lock() {
            Ok(handoff) => handoff,
            Err(TryLockError::WouldBlock | TryLockError::Poisoned(_)) => return,
        };
        self.snapshot = handoff.snapshot();
        if let Some((revision, legend)) = handoff.legend_since(self.legend_revision) {
            self.legend = legend;
            self.legend_revision = revision;
        }
    }
}

impl FrameHandler for FreecamFrameHandler {
    fn frame(&mut self, ctx: &Context, input: &FrameInputView) -> FrameReply {
        match input.layer() {
            LayerKind::Hud => FrameReply { cursor: CursorShape::Default, layer_rect: Rect::NOTHING },
            LayerKind::Overlay => self.overlay(ctx, input),
        }
    }

    fn textures_lost(&mut self) {
        self.ui.textures_lost();
    }
}
