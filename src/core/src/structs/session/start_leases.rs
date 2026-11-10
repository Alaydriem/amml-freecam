use amm_core::structs::descriptor::Capability;

// how the mod manager answered the flight lease requests of a start
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartLeases {
    Granted,
    // the first capability another mod holds; every lease granted before it was given back
    Denied(Capability),
}
