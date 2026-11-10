use std::cell::Cell;

thread_local! {
    static DELIVERING: Cell<bool> = const { Cell::new(false) };
}

// marks the mod's own camera instruction delivery on this thread, so the instruction detour does not report it as
// the server's; cleared when the scope ends, on every exit
pub struct DeliveringScope {
    _private: (),
}

impl DeliveringScope {
    pub fn enter() -> Self {
        DELIVERING.with(|flag| flag.set(true));
        Self { _private: () }
    }

    // whether this thread is inside a delivery of the mod
    pub fn active() -> bool {
        DELIVERING.with(Cell::get)
    }
}

impl Drop for DeliveringScope {
    fn drop(&mut self) {
        DELIVERING.with(|flag| flag.set(false));
    }
}
