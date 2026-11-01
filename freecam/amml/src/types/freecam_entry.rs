use amml_game::EntryId;

// the entries of the freecam-only game access; the shared ones are in amml-game. Only what moves between builds is an
// entry: functions, vtable slots, the offset of a struct in its owner, and the size checks of the struct mirrors
pub struct FreecamEntry;

impl FreecamEntry {
    // delivery: the client packet handler, its handle overloads and the camera instruction packet
    pub const CLIENT_NETWORK_HANDLER_VTABLE: EntryId = EntryId::new("ClientNetworkHandler.vtable");
    pub const HANDLE_CHUNK_PUBLISHER: EntryId =
        EntryId::new("ClientNetworkHandler.handle.NetworkChunkPublisherUpdatePacket");
    pub const HANDLE_CAMERA_PRESETS: EntryId = EntryId::new("ClientNetworkHandler.handle.CameraPresetsPacket");
    pub const HANDLE_CAMERA_INSTRUCTION: EntryId = EntryId::new("ClientNetworkHandler.handle.CameraInstructionPacket");
    pub const CREATE_PACKET: EntryId = EntryId::new("MinecraftPackets.createPacket");
    pub const INSTRUCTION_PACKET_INSTRUCTION: EntryId = EntryId::new("CameraInstructionPacket.mCameraInstruction");
    pub const INSTRUCTION_PACKET_SIZE: EntryId = EntryId::new("CameraInstructionPacket.size");
    pub const INSTRUCTION_SPLINE: EntryId = EntryId::new("CameraInstruction.mSpline");
    pub const LEVEL_GET_CAMERA_PRESETS: EntryId = EntryId::new("Level.getCameraPresets");
    pub const CAMERA_PRESET_SIZE: EntryId = EntryId::new("CameraPreset.size");

    // markers: the primitive shapes packet and its payload
    pub const HANDLE_PRIMITIVE_SHAPES: EntryId = EntryId::new("ClientNetworkHandler.handle.PrimitiveShapesPacket");
    pub const SHAPES_PACKET_SHAPES: EntryId = EntryId::new("PrimitiveShapesPacket.mShapes");
    pub const PAYLOAD_SIZE: EntryId = EntryId::new("PrimitiveShapeDataPayload.size");

    // camera watch: the camera registry and the entt type ids of the camera components
    pub const GET_CAMERA_REGISTRY: EntryId = EntryId::new("ClientInstance.getCameraRegistry");
    pub const CAMERA_ENTITIES: EntryId = EntryId::new("CameraRegistry.mCameraEntities");
    pub const CAMERA_ENTITY_STRIDE: EntryId = EntryId::new("CameraRegistry.cameraEntityStride");
    pub const CAMERA_COMPONENT_TYPE_ID: EntryId = EntryId::new("CameraComponent.typeId");
    pub const CAMERA_COMPONENT_SIZE_FACTOR: EntryId = EntryId::new("CameraComponent.sizeFactor");
    pub const CAMERA_COMPONENT_SIZE_SHIFT: EntryId = EntryId::new("CameraComponent.sizeShift");
    pub const ACTIVE_CAMERA_TYPE_ID: EntryId = EntryId::new("ActiveCameraComponent.typeId");

    // look-at: the block ray and the actor boxes
    pub const BLOCK_SOURCE_CLIP: EntryId = EntryId::new("BlockSource.clip");
    pub const BLOCK_SOURCE_FETCH_ENTITIES: EntryId = EntryId::new("BlockSource.fetchEntities");
    pub const ACTOR_AABB_SHAPE: EntryId = EntryId::new("Actor.aabbShapeComponent");
}
