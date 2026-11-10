use amml_game::{EntryId, GameEntries};

use crate::types::FreecamEntry;

// the parts of the mod. `delivery` is required (freecam cannot move the game camera without it); the rest are
// optional and turn off alone when one of their entries did not resolve
pub struct FreecamFeature;

impl FreecamFeature {
    // the camera instruction to the client's own packet handler, and the capture of that handler
    pub const DELIVERY: &'static str = "delivery";
    // path markers through the primitive shapes packet
    pub const MARKERS: &'static str = "markers";
    // which camera the game shows, from the camera registry (without it, from the server's camera instructions)
    pub const CAMERA_WATCH: &'static str = "camera watch";
    // the look ray: blocks through BlockSource::clip, actors through their boxes
    pub const LOOK_AT: &'static str = "look-at";

    pub const OPTIONAL: [&'static str; 3] = [Self::MARKERS, Self::CAMERA_WATCH, Self::LOOK_AT];

    pub const DELIVERY_ENTRIES: [EntryId; 10] = [
        FreecamEntry::CLIENT_NETWORK_HANDLER_VTABLE,
        FreecamEntry::HANDLE_CHUNK_PUBLISHER,
        FreecamEntry::HANDLE_CAMERA_PRESETS,
        FreecamEntry::HANDLE_CAMERA_INSTRUCTION,
        FreecamEntry::CREATE_PACKET,
        FreecamEntry::INSTRUCTION_PACKET_INSTRUCTION,
        FreecamEntry::INSTRUCTION_PACKET_SIZE,
        FreecamEntry::INSTRUCTION_SPLINE,
        FreecamEntry::LEVEL_GET_CAMERA_PRESETS,
        FreecamEntry::CAMERA_PRESET_SIZE,
    ];

    pub fn entries(feature: &str) -> &'static [EntryId] {
        match feature {
            Self::DELIVERY => &Self::DELIVERY_ENTRIES,
            Self::MARKERS => {
                &[FreecamEntry::HANDLE_PRIMITIVE_SHAPES, FreecamEntry::SHAPES_PACKET_SHAPES, FreecamEntry::PAYLOAD_SIZE]
            }
            Self::CAMERA_WATCH => &[
                FreecamEntry::GET_CAMERA_REGISTRY,
                FreecamEntry::CAMERA_ENTITIES,
                FreecamEntry::CAMERA_ENTITY_STRIDE,
                FreecamEntry::CAMERA_COMPONENT_TYPE_ID,
                FreecamEntry::CAMERA_COMPONENT_SIZE_FACTOR,
                FreecamEntry::CAMERA_COMPONENT_SIZE_SHIFT,
                FreecamEntry::ACTIVE_CAMERA_TYPE_ID,
            ],
            Self::LOOK_AT => &[
                FreecamEntry::BLOCK_SOURCE_CLIP,
                FreecamEntry::BLOCK_SOURCE_FETCH_ENTITIES,
                FreecamEntry::ACTOR_AABB_SHAPE,
            ],
            _ => &[],
        }
    }

    // whether every entry of the feature resolved
    pub fn is_on(entries: &GameEntries, feature: &str) -> bool {
        Self::entries(feature).iter().all(|id| entries.get(*id).is_some())
    }
}
