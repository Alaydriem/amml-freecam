use amml_api::amml_abi::{
    AmmlPhase, CameraInstructionReceived, CameraSetup, ClientTick, KeyInput, MouseInput, WindowFocus, WorldLeft,
};
use amml_api::{ApiError, CameraInstructionReceivedExt, Ctx, Event, HookOrder, Listener, WindowFocusExt};

use crate::services::FreecamRuntime;

// the loader listeners of freecam; each only hands its event to the runtime. Dropping them removes them
pub struct FreecamListeners {
    listeners: Vec<Listener>,
}

impl FreecamListeners {
    pub const COUNT: usize = 8;

    pub fn start(ctx: &Ctx, runtime: &'static FreecamRuntime) -> Result<Self, ApiError> {
        let events = ctx.events();
        let mut listeners = Vec::with_capacity(Self::COUNT);
        listeners.push(events.listen::<ClientTick>(HookOrder::READ_AFTER_WRITERS, move |_, at| runtime.tick(at))?);
        listeners.push(events.listen::<CameraSetup>(HookOrder::WRITE, move |event, at| {
            if event.header().phase == AmmlPhase::BEFORE {
                runtime.before_camera_setup(event, at);
            }
        })?);
        listeners.push(events.listen::<CameraSetup>(HookOrder::READ_AFTER_WRITERS, move |event, _| {
            if event.header().phase == AmmlPhase::AFTER {
                runtime.after_camera_setup(event);
            }
        })?);
        listeners.push(events.listen::<CameraInstructionReceived>(HookOrder::NORMAL, move |event, _| {
            runtime.camera_instruction(event.kind(), event.header().origin);
        })?);
        let keys = *ctx;
        listeners.push(events.listen::<KeyInput>(HookOrder::NORMAL, move |event, _| runtime.key(event, &keys))?);
        listeners.push(events.listen::<MouseInput>(HookOrder::NORMAL, move |event, _| runtime.mouse(event))?);
        listeners.push(events.listen::<WorldLeft>(HookOrder::NORMAL, move |_, _| runtime.world_left())?);
        let log = ctx.log();
        listeners.push(events.listen::<WindowFocus>(HookOrder::NORMAL, move |event, _| {
            runtime.focus(event.gained(), event.replayed(), &log);
        })?);
        Ok(Self { listeners })
    }

    // removes every listener; when this returns no listener call runs
    pub fn stop(self) {
        drop(self.listeners);
    }
}
