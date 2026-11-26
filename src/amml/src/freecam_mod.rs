use std::sync::{Mutex, PoisonError};

use amml_api::amml_abi::{
    CameraInstructionReceived, CameraSetup, ClientTick, InjectedInputReleased, InputLeaseLost, InputLeaseResumed,
    InputLeaseSuspended, KeyInput, ModsLoaded, MouseInput, ProviderGone, ProviderReady, ScreenshotTaken,
    ScreenshotWritten, WindowFocus, WorldLeft,
};
use amml_api::{AmmlMod, ApiError, Area, Ctx, DisableCtx, Uses};

use crate::services::{FrameDriver, FreecamListeners, FreecamRuntime, FreecamSession};

// what the enabled mod owns besides the runtime. `disable` takes it, which removes the listeners
static LISTENERS: Mutex<Option<FreecamListeners>> = Mutex::new(None);

// the AMML entry point: enable starts the manager link, the frame driver and the listeners; disable removes the
// listeners and drops the driver, which closes the link. The loader releases the free camera and removes the shapes
// of a disabled mod itself
pub struct FreecamMod;

impl AmmlMod for FreecamMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "freecam";
    const USES: Uses = amml_api::amml_uses! {
        feature "flight" {
            required ClientTick;
            required CameraSetup;
            required CameraInstructionReceived;
            required KeyInput;
            required MouseInput;
            required WorldLeft;
            required WindowFocus;
            required ProviderReady;
            required ProviderGone;
            required ModsLoaded;
            required InputLeaseSuspended;
            required InputLeaseResumed;
            required InputLeaseLost;
            required InjectedInputReleased;
            required area Area::CLIENT_READ;
            required area Area::INPUT_LEASE;
            required area Area::ACTOR_READ.at(3);
            required area Area::ACTOR_HEALTH;
            required area Area::CAMERA_POSE;
            required area Area::CAMERA_PROJECTION;
            required area Area::CAMERA_FREE;
        }
        feature "markers" {
            required area Area::WORLD_SHAPES;
        }
        feature "camera_watch" {
            required area Area::CAMERA_ACTIVE;
        }
        feature "look_at" {
            required area Area::WORLD_RAYCAST;
            required area Area::WORLD_ACTORS_IN;
        }
        feature "screenshots" {
            required ScreenshotTaken;
            required ScreenshotWritten;
            required area Area::OVERLAY_SCREENSHOT;
        }
    };

    fn enable(ctx: &Ctx) -> Result<(), ApiError> {
        let log = ctx.log();
        let session = FreecamSession::start(ctx).map_err(|error| {
            log.error("freecam off", &[("error", &error.to_string())]);
            ApiError::Refused
        })?;
        let runtime = FreecamRuntime::get();
        runtime.start(FrameDriver::new(*ctx, session));
        match FreecamListeners::start(ctx, runtime) {
            Ok(listeners) => {
                *LISTENERS.lock().unwrap_or_else(PoisonError::into_inner) = Some(listeners);
                Ok(())
            }
            Err(error) => {
                runtime.stop();
                log.error("freecam off", &[("error", &error.to_string())]);
                Err(error)
            }
        }
    }

    // the listeners go first, outside the lock, so no frame runs while the driver and the link close
    fn disable(_ctx: &Ctx, _how: &DisableCtx) {
        let listeners = LISTENERS.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(listeners) = listeners {
            listeners.stop();
        }
        FreecamRuntime::get().stop();
    }
}
