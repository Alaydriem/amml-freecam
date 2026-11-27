#![allow(clippy::unwrap_used, clippy::expect_used)]

mod freecam_mod;
mod services;
mod support;
mod types;

#[global_allocator]
static GLOBAL: &stats_alloc::StatsAlloc<std::alloc::System> = &stats_alloc::INSTRUMENTED_SYSTEM;
