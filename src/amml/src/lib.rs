//! @alaydriem/freecam as an AMML mod: drives freecam-core from the loader's client tick and camera setup events, moves
//! the game camera through the loader's free camera, and draws the path markers as world shapes.

#![deny(unsafe_code)]

pub mod freecam_mod;
pub mod services;
pub mod types;

pub use freecam_mod::FreecamMod;

amml_api::amml_mod!(FreecamMod);
