use amml_api::{ApiError, Rgba, ShapeOp, WorldShapes};
use freecam_core::structs::{MarkerEntry, MarkerKind};

use crate::services::PoseConvert;

// delivers core's marker batches as world shapes: one `apply` per run of one dimension, at most `max_batch` ops each.
// The time the last batch took goes back to core with the next status.
//
// Bound: `ops` grows to the largest run sent (at most `max_batch`, 800) and is reused for every run after
pub struct MarkerDelivery {
    ops: Vec<ShapeOp>,
    last_send_us: f32,
    // the error of the last failed apply, so a repeated refusal gives one line
    last_error: Option<ApiError>,
}

impl MarkerDelivery {
    pub fn new() -> Self {
        Self { ops: Vec::new(), last_send_us: 0.0, last_error: None }
    }

    // the generation of the loader's delivery target; 0 while the loader has none
    pub fn target_epoch(shapes: &WorldShapes) -> u32 {
        shapes.target_epoch().unwrap_or(0)
    }

    // microseconds the last delivered batch took, 0 when nothing went out
    pub fn last_send_us(&self) -> f32 {
        self.last_send_us
    }

    // sends `markers`; the payloads the loader handed to the game, or None when nothing was sent
    pub fn deliver(&mut self, shapes: &WorldShapes, markers: &[MarkerEntry]) -> Option<(u32, i32)> {
        self.last_send_us = 0.0;
        if markers.is_empty() {
            return None;
        }
        let mut delivered = 0u32;
        let mut elapsed = 0u32;
        let mut dimension = 0;
        let mut last_error = self.last_error;
        self.for_each_batch(markers, shapes.max_batch(), |batch_dimension, ops| {
            match shapes.apply(batch_dimension, ops) {
                Ok(stats) => {
                    delivered = delivered.saturating_add(stats.delivered);
                    elapsed = elapsed.saturating_add(stats.elapsed_us);
                    dimension = batch_dimension;
                    last_error = None;
                }
                Err(error) => {
                    if last_error != Some(error) {
                        curia::warn!("markers not delivered", { error: error.to_string(), ops: ops.len() });
                    }
                    last_error = Some(error);
                }
            }
        });
        self.last_error = last_error;
        self.last_send_us = elapsed as f32;
        Some((delivered, dimension))
    }

    // calls `send` with each run of `markers` as ops: split at each dimension change and at `max_batch`, in one
    // buffer that is reused
    pub fn for_each_batch(&mut self, markers: &[MarkerEntry], max_batch: usize, mut send: impl FnMut(i32, &[ShapeOp])) {
        let limit = max_batch.max(1);
        let mut dimension = markers.first().map_or(0, |marker| marker.dimension);
        self.ops.clear();
        for marker in markers {
            if !self.ops.is_empty() && (marker.dimension != dimension || self.ops.len() >= limit) {
                send(dimension, &self.ops);
                self.ops.clear();
            }
            dimension = marker.dimension;
            self.ops.push(Self::op(marker));
        }
        if !self.ops.is_empty() {
            send(dimension, &self.ops);
        }
    }

    fn op(marker: &MarkerEntry) -> ShapeOp {
        let shape = marker.shape;
        let (id, from, to, color) = (
            marker.id,
            PoseConvert::vec_to_api(shape.location),
            PoseConvert::vec_to_api(shape.end),
            Self::color(shape.color),
        );
        match shape.kind {
            MarkerKind::Remove => ShapeOp::Remove { id },
            MarkerKind::Line => ShapeOp::Line { id, from, to, color },
            MarkerKind::Box => ShapeOp::Box { id, from, to, color },
            MarkerKind::Arrow => ShapeOp::Arrow { id, from, to, color },
        }
    }

    // core's colours are ARGB
    fn color(argb: u32) -> Rgba {
        let [a, r, g, b] = argb.to_be_bytes();
        Rgba { r, g, b, a }
    }
}

impl Default for MarkerDelivery {
    fn default() -> Self {
        Self::new()
    }
}
