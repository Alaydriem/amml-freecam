mod id;
mod message;
mod rules;
mod runtime;
mod support;
mod text;
mod views;

#[global_allocator]
static GLOBAL: support::counting_alloc::CountingAlloc = support::counting_alloc::CountingAlloc::system();
