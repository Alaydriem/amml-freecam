//! @alaydriem/freecam as an AMML mod: drives freecam-core from the client update and the render camera, moves the
//! game camera through the client's own camera instructions, and draws the path markers as primitive shapes.

#![deny(unsafe_code)]

pub mod api;
pub mod detours;
pub mod freecam_mod;
pub mod game;
pub mod services;
pub mod types;

pub use freecam_mod::FreecamMod;

amml_sdk::amml_mod!(FreecamMod);
