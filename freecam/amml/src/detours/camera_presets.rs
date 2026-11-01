use amml_sdk::DetourGuard;

use crate::detours::handle_fn::HandleFn;
use crate::services::FreecamRuntime;

// void ClientNetworkHandler::handle(NetworkIdentifier const&, CameraPresetsPacket const&): a packet the server always
// sends; captures the handler and its id before the original
pub static GUARD: DetourGuard = DetourGuard::new("camera_presets");

#[allow(unsafe_code)]
pub extern "C" fn detour(this: usize, id: usize, packet: usize) {
    let original: HandleFn = GUARD.original_fn();
    if let Some(runtime) = FreecamRuntime::get() {
        runtime.on_handle(this, id);
    }
    // SAFETY: `original` is the next function in the chain, with the hooked signature; the arguments are the game's
    unsafe { original(this, id, packet) };
}
