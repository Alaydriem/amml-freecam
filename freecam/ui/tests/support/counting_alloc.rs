use std::alloc::System;

use stats_alloc::StatsAlloc;

/// The system allocator with allocation counters, registered as the global allocator of the test binary.
///
/// `stats_alloc` holds the `unsafe impl GlobalAlloc`, so this crate keeps no unsafe code of its own.
pub type CountingAlloc = StatsAlloc<System>;
