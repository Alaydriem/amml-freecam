use windows_sys::Win32::System::Threading::GetCurrentThreadId;

// the Windows id of a thread, as the game log and the capture name it
pub struct ThreadId;

impl ThreadId {
    #[allow(unsafe_code)]
    pub fn current() -> u32 {
        // SAFETY: GetCurrentThreadId takes no arguments and cannot fail
        unsafe { GetCurrentThreadId() }
    }
}
