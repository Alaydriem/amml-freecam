// The shape of a ClientNetworkHandler handle detour: `(this, id, packet)`.
//
// The game declares `void handle(NetworkIdentifier const&, <Packet> const&)` for each packet type; every overload has
// this shape
pub type HandleFn = unsafe extern "C" fn(usize, usize, usize);
