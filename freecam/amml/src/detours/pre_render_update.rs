use amml_game::hooks::PreRenderUpdateFn;
use amml_sdk::DetourGuard;

use crate::services::FreecamRuntime;

// void LevelRendererCamera::preRenderUpdate(ScreenContext&, LevelRenderPreRenderUpdateParameters&): the base function
// the player's override calls first. The free camera's position is written after it, before the chunk sort
pub static GUARD: DetourGuard = DetourGuard::new("pre_render_update");

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, screen_ctx: usize, params: usize) {
    let original: PreRenderUpdateFn = GUARD.original_fn();
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    unsafe { original(this, screen_ctx, params) };
    if let Some(runtime) = FreecamRuntime::get() {
        runtime.after_pre_render_update(this);
    }
}
