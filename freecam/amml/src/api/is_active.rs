use crate::services::FreecamRuntime;

/// True while freecam controls the camera, from turning on until the controller is fully off. Reads one atomic
/// flag: safe to call from any thread, every frame.
// SAFETY: the name is unique to this mod and the function takes no arguments
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn alaydriem_freecam_is_active() -> bool {
    FreecamRuntime::get().is_some_and(FreecamRuntime::is_active)
}
