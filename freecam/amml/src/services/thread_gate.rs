use std::sync::atomic::{AtomicU32, Ordering};

use crate::types::ThreadId;

// lets game work run only on the thread that handles the server's packets; the first refusal is logged with both
// thread ids, later ones are not
pub struct ThreadGate {
    site: &'static str,
    warnings: AtomicU32,
}

impl ThreadGate {
    pub const fn new(site: &'static str) -> Self {
        Self { site, warnings: AtomicU32::new(0) }
    }

    // whether the current thread is `handler_thread`
    pub fn allows(&self, handler_thread: u32) -> bool {
        let current = ThreadId::current();
        if current == handler_thread {
            return true;
        }
        if self.warnings.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            curia::warn!("thread differs from the packet handler thread; the call is refused", {
                site: self.site,
                thread: current,
                handler_thread: handler_thread,
            });
        }
        false
    }

    // the number of refusal lines logged: 0 or 1
    pub fn warnings(&self) -> u32 {
        self.warnings.load(Ordering::Relaxed)
    }

    // a new session logs its first refusal again
    pub fn reset(&self) {
        self.warnings.store(0, Ordering::Relaxed);
    }
}
