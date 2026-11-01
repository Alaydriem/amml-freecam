use amml_game::hooks::SetupCameraFn;
use amml_sdk::DetourGuard;

use crate::services::FreecamRuntime;

// void LevelRendererPlayer::setupCamera(mce::Camera&, float): once per rendered frame. The frame runs before the
// original; the free camera's rotation is written after it, before the readers that run after the writers
pub static GUARD: DetourGuard = DetourGuard::new("setup_camera");

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, camera: usize, alpha: f32) {
    let original: SetupCameraFn = GUARD.original_fn();
    let runtime = FreecamRuntime::get();
    if let Some(runtime) = runtime {
        runtime.before_setup_camera();
    }
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    unsafe { original(this, camera, alpha) };
    if let Some(runtime) = runtime {
        runtime.after_setup_camera(camera);
    }
}
