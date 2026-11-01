use amm_core::structs::descriptor::Capability;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartRefusal {
    NotInWorld,
    MenuOpen,
    NotReady,
    ServerCameraActive,
    // the mod manager gave this capability to another mod
    LeaseDenied(Capability),
}
