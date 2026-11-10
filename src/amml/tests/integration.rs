#![allow(clippy::unwrap_used, clippy::expect_used)]

mod api;
mod freecam_mod;
mod game;
mod services;
mod support;
mod types;

#[global_allocator]
static GLOBAL: support::counting_alloc::CountingAlloc = support::counting_alloc::CountingAlloc::system();
