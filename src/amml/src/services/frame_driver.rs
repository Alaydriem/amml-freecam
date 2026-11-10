use std::time::Instant;

use amml_game::{Actor, ClientInstance, StopLatch};
use freecam_core::structs::{CameraCommand, FrameOutput, FrameStatus, MarkerEntry};

use crate::services::FrameStatusReader;
use crate::types::{FrameContext, ThreadId};

// runs core once per frame and carries out what it asks: the marker batch, the look ray, the camera instruction and
// the render override. In a world the frame runs before setupCamera; outside one it runs from the client update
pub struct FrameDriver {
    // the frame's marker batch; sized once, at the session's marker capacity
    markers: Vec<MarkerEntry>,
    // the time the last batch took to deliver, for the next status; 0 when nothing went out
    last_shape_send_us: f32,
    setup_thread_logged: bool,
    update_thread_logged: bool,
    tick_thread_logged: bool,
}

impl FrameDriver {
    pub fn new(marker_capacity: usize) -> Self {
        Self {
            markers: Vec::with_capacity(marker_capacity),
            last_shape_send_us: 0.0,
            setup_thread_logged: false,
            update_thread_logged: false,
            tick_thread_logged: false,
        }
    }

    // before setupCamera, in a world or out of one
    pub fn before_setup_camera(
        &mut self,
        context: FrameContext<'_>,
        client: &ClientInstance<'_>,
    ) -> Option<FrameOutput> {
        Self::note_thread(&mut self.setup_thread_logged, "the frame (setupCamera)");
        let player = client.local_player();
        context.capture.observe_local_player(player.is_some());
        let rendered = context.render.take_rendered();
        let game = context.game;
        let status = context.reader.read(
            client,
            &game.instructions,
            context.capture,
            &game.watch,
            rendered,
            context.camera_owned,
        );
        let output = self.run(context, status)?;
        self.answer_ray(context, &output, player.as_ref());
        Self::apply_camera(context, &output);
        context.render.set(output.render_override);
        Some(output)
    }

    // after the client update: outside a world the frame runs here, since setupCamera does not run then
    pub fn tick(&mut self, context: FrameContext<'_>, client: &ClientInstance<'_>) -> Option<FrameOutput> {
        Self::note_thread(&mut self.update_thread_logged, "the client update");
        let player = client.local_player();
        if player.is_some() && client.is_world_active() {
            return None;
        }
        Self::note_thread(&mut self.tick_thread_logged, "the frame (client update)");
        context.capture.observe_local_player(player.is_some());
        let status = FrameStatusReader::outside_world(
            FrameStatusReader::now_seconds(),
            FrameStatusReader::game_window_focused(),
        );
        let output = self.run(context, status)?;
        // no world is active here, so there is nothing to cast through
        self.answer_ray(context, &output, None);
        context.render.set(output.render_override);
        Some(output)
    }

    // disable, before the hooks go: the camera goes back to the player (when freecam holds it) and every live marker
    // is removed. Nothing goes out once the stop latch is raised (the game is closing, or a hook body panicked), when
    // no local player is there, no handler is captured or this is not the capture thread. Returns whether the clear
    // went out and how many removes did
    pub fn shutdown(
        &mut self,
        context: FrameContext<'_>,
        client: Option<&ClientInstance<'_>>,
        camera_owned: bool,
        latch: &StopLatch,
    ) -> (bool, usize) {
        if latch.raised() {
            curia::info!("shutdown: the stop latch is raised; nothing is sent");
            return (false, 0);
        }
        context.session.shutdown_markers(&mut self.markers);
        let world_live = client.is_some_and(|client| client.local_player().is_some());
        if !world_live {
            if camera_owned || !self.markers.is_empty() {
                curia::warn!("shutdown: no world, so the camera and the markers stay as they are", {
                    markers: self.markers.len(),
                });
            }
            return (false, 0);
        }
        let cleared = camera_owned && context.game.instructions.send_clear(context.capture);
        let removed = match &context.game.shapes {
            Some(shapes) if !self.markers.is_empty() && shapes.deliver(context.capture, &self.markers) => {
                self.markers.len()
            }
            _ => 0,
        };
        if removed < self.markers.len() {
            curia::warn!("shutdown: markers not removed (no captured handler, or another thread)", {
                markers: self.markers.len(),
            });
        }
        curia::info!("shutdown", { camera_cleared: cleared, markers_removed: removed });
        (cleared, removed)
    }

    // asks core for this frame with the delivery status, then delivers its marker batch
    fn run(&mut self, context: FrameContext<'_>, mut status: FrameStatus) -> Option<FrameOutput> {
        status.last_shape_send_us = self.last_shape_send_us;
        status.handler_generation = context.capture.delivery_generation();
        let output = context.session.frame(&status, &mut self.markers)?;
        self.deliver_markers(context);
        Some(output)
    }

    fn deliver_markers(&mut self, context: FrameContext<'_>) {
        let mut elapsed = 0.0f32;
        if !self.markers.is_empty()
            && let Some(shapes) = &context.game.shapes
        {
            let start = Instant::now();
            if shapes.deliver(context.capture, &self.markers) {
                elapsed = start.elapsed().as_secs_f32() * 1_000_000.0;
            }
        }
        self.last_shape_send_us = elapsed;
    }

    // answers the ray this frame asked for; with no local player, no block source or look-at off, a miss
    #[allow(unsafe_code)]
    fn answer_ray(&self, context: FrameContext<'_>, output: &FrameOutput, player: Option<&Actor<'_>>) {
        let Some(ray) = output.ray else { return };
        let hit = match (player, &context.game.look) {
            (Some(player), Some(look)) => player.block_source().and_then(|region| {
                // SAFETY: the live local player of this frame and its block source, on the game thread
                unsafe {
                    look.cast(
                        context.capture,
                        player.as_ptr(),
                        region.as_ptr(),
                        ray.origin,
                        ray.direction,
                        ray.max_distance,
                    )
                }
            }),
            _ => None,
        };
        let _ = context.session.ray_result(hit);
    }

    fn apply_camera(context: FrameContext<'_>, output: &FrameOutput) {
        let instructions = &context.game.instructions;
        match output.camera {
            CameraCommand::Set(pose) => {
                instructions.send_set(context.capture, &pose);
            }
            CameraCommand::Clear => {
                instructions.send_clear(context.capture);
            }
            CameraCommand::None => {}
        }
    }

    // one line per call site with the id of the thread that runs it
    fn note_thread(logged: &mut bool, site: &str) {
        if *logged {
            return;
        }
        *logged = true;
        curia::info!("thread of a call site", { site: site, thread: ThreadId::current() });
    }
}
