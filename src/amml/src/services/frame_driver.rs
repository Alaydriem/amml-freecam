use std::sync::atomic::{AtomicBool, Ordering};

use amml_api::amml_abi::{AmmlOrigin, CameraSetup};
use amml_api::{CameraSetupExt, Client, Ctx, EventCtx, InstructionKind, Logger};
use freecam_core::structs::{CameraPose, FrameOutput, FrameStatus, MarkerEntry, ServerInstruction};

use crate::services::{
    CameraDriver, CameraWatch, FlightProbe, FrameStatusReader, FreecamSession, InputBridge, LookRay, MarkerDelivery,
};
use crate::types::InputItem;

// runs core once per frame and carries out what it asks: the marker batch, the look ray, the free camera and the
// rendered pose. In a world the frame runs in `CameraSetup` Before (game thread); outside one it runs from
// `ClientTick`. Everything here is game thread, behind the runtime's lock
pub struct FrameDriver {
    ctx: Ctx,
    log: Logger,
    session: FreecamSession,
    camera: CameraDriver,
    delivery: MarkerDelivery,
    watch: CameraWatch,
    probe: FlightProbe,
    // the frame's marker batch; sized once, at the session's marker capacity
    markers: Vec<MarkerEntry>,
    inputs: Vec<InputItem>,
    // the pose of the running frame, for a second camera setup in the same frame
    pose: Option<CameraPose>,
    camera_owned: bool,
    off_thread_logged: bool,
}

impl FrameDriver {
    pub fn new(ctx: Ctx, session: FreecamSession) -> Self {
        let capacity = session.marker_capacity();
        Self {
            ctx,
            log: ctx.log(),
            session,
            camera: CameraDriver::new(),
            delivery: MarkerDelivery::new(),
            watch: CameraWatch::new(),
            probe: FlightProbe::new(),
            markers: Vec::with_capacity(capacity),
            inputs: Vec::with_capacity(crate::services::InputQueue::CAPACITY),
            pose: None,
            camera_owned: false,
            off_thread_logged: false,
        }
    }

    // `CameraSetup` Before: the world frame, once per frame; a later setup of the same frame gets the same pose
    pub fn world_frame(
        &mut self,
        event: &CameraSetup,
        at: &EventCtx,
        reader: &FrameStatusReader,
        input: &InputBridge,
        focus_lost: &AtomicBool,
    ) {
        // `None` only from a loader without the camera table: the frame then runs without the rendered pose
        let camera = event.camera();
        if !event.first_in_frame() {
            if let Some(camera) = &camera {
                self.camera.set_pose(camera, self.pose, &self.log);
            }
            return;
        }
        let Some(client) = at.client() else {
            if !self.off_thread_logged {
                self.off_thread_logged = true;
                self.log.warn("camera setup is off the game thread; the world frame does not run", &[]);
            }
            return;
        };
        let control = self.ctx.camera();
        let kind = control.active_camera(&client);
        let player_camera = self.watch.player_camera_active(kind);
        if let Some(name) = self.watch.take_changed() {
            self.log.info("camera watch", &[("kind", name)]);
        }
        let free_available = control.free_available() == Some(true);
        let rendered = camera.as_ref().and_then(|camera| camera.rendered_position());
        let focused = Self::take_focus(focus_lost);
        let mut status = reader.read(&client, focused, free_available, player_camera, rendered);
        let Some(output) = self.run(&mut status, input) else { return };
        self.answer_ray(&output, Some(&client));
        self.camera.command(&control, output.camera, &self.log);
        self.pose = output.render_override;
        if let Some(camera) = &camera {
            self.camera.set_pose(camera, self.pose, &self.log);
        }
    }

    // `ClientTick`: outside a world the frame runs here, since no camera setup comes then
    pub fn outside_frame(
        &mut self,
        at: &EventCtx,
        reader: &FrameStatusReader,
        input: &InputBridge,
        focus_lost: &AtomicBool,
    ) {
        if let Some(client) = at.client()
            && Self::in_world(&client)
        {
            return;
        }
        let mut status = reader.outside_world(Self::take_focus(focus_lost));
        let Some(output) = self.run(&mut status, input) else { return };
        // no world is active here, so there is nothing to cast through
        self.answer_ray(&output, None);
        self.camera.command(&self.ctx.camera(), output.camera, &self.log);
        self.pose = output.render_override;
    }

    // a camera instruction reached the client handler. The server's (`GAME`) end the flight; this mod's own echo is
    // not passed on; another mod's free camera call is passed as not the server's
    pub fn camera_instruction(&mut self, kind: InstructionKind, origin: AmmlOrigin) {
        if origin == AmmlOrigin::THIS_MOD {
            return;
        }
        let from_game = origin == AmmlOrigin::GAME;
        if from_game {
            self.watch.on_server_instruction(kind);
        }
        let kind = match kind {
            InstructionKind::Set | InstructionKind::Clear => ServerInstruction::SetOrClear,
            _ => ServerInstruction::Other,
        };
        if from_game && kind == ServerInstruction::SetOrClear && self.camera_owned {
            self.log.info("flight stopped", &[("reason", "server_camera"), ("origin", "game")]);
        }
        let _ = self.session.server_instruction(kind, !from_game);
    }

    // camera instructions dropped at the bound of the runtime's list since the last frame
    pub fn instructions_dropped(&self, count: u32) {
        self.log.warn("camera instructions dropped", &[("count", &count.to_string())]);
    }

    // `WorldLeft`: the loader forgot the free camera, the pose and the shapes; the server camera goes with the world
    pub fn world_left(&mut self) {
        self.camera.forget();
        self.watch.reset();
        self.pose = None;
    }

    // asks core for this frame with the delivery status, then delivers its marker batch and logs the probes
    fn run(&mut self, status: &mut FrameStatus, input: &InputBridge) -> Option<FrameOutput> {
        let shapes = self.ctx.world_shapes();
        status.last_shape_send_us = self.delivery.last_send_us();
        status.handler_generation = MarkerDelivery::target_epoch(&shapes);
        if !status.focused {
            input.release_held();
        }
        input.drain(&mut self.inputs);
        let frame = self.session.frame(status, &self.inputs, input.lingering(), &mut self.markers)?;
        if let Some(keys) = &frame.keys {
            input.set_keys(keys);
        }
        input.set_gate(frame.holding, frame.panel_open);
        if status.in_world
            && let Some((delivered, dimension)) = self.delivery.deliver(&shapes, &self.markers)
            && delivered > 0
        {
            self.log.info(
                "markers applied",
                &[("delivered", &delivered.to_string()), ("dimension", &dimension.to_string())],
            );
        }
        let output = frame.output;
        let position = output.render_override.map(|pose| pose.position);
        self.probe.inputs(&self.inputs, position, &self.log);
        self.probe.points(frame.point_count, &self.log);
        self.probe.focus(status.focused, output.camera_owned, &self.log);
        self.camera_owned = output.camera_owned;
        Some(output)
    }

    // answers the ray this frame asked for; with no local player or no block source, a miss
    fn answer_ray(&mut self, output: &FrameOutput, client: Option<&Client<'_>>) {
        let Some(ray) = output.ray else { return };
        let source = client.and_then(|client| client.local_player()).and_then(|player| player.block_source());
        let hit = source.and_then(|source| LookRay::cast(&source, ray.origin, ray.direction, ray.max_distance));
        self.log.info("look-at ray answered", &[("hit", if hit.is_some() { "block_or_actor" } else { "none" })]);
        let _ = self.session.ray_result(hit);
    }

    // false once after a focus loss: only a frame that runs core takes the loss, so a client tick in a world (which
    // runs no frame) does not drop it
    fn take_focus(focus_lost: &AtomicBool) -> bool {
        !focus_lost.swap(false, Ordering::Relaxed)
    }

    fn in_world(client: &Client<'_>) -> bool {
        client.world_active() == Some(true) && client.local_player().is_some()
    }
}
