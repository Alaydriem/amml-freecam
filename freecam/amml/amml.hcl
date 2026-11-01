namespace   = "alaydriem"
name        = "freecam"
entry       = "alaydriem_freecam.dll"
api_version = 2
entries = [
  # required: freecam refuses to enable without them
  "ClientInstance.update",
  "ClientInstance.getLocalPlayer",
  "ClientInstance.isWorldActive",
  "ClientInstance.isShowingMenu",
  "Actor.stateVectorComponent",
  "Actor.actorRotationComponent",
  "StateVectorComponent.mPos",
  "ActorRotationComponent.mRot",
  "Actor.mLevel",
  "Actor.getDimension",
  "Dimension.getDimensionId",
  "Actor.getDimensionBlockSource",
  "LevelRendererPlayer.setupCamera",
  "LevelRendererPlayer.createViewRenderObject",
  # the camera write (game.camera_write; Camera.mForward anchors the Camera mirror and the projection read)
  "Camera.mForward",
  "Camera.updateViewMatrixDependencies",
  "LevelRendererCamera.preRenderUpdate",
  "LevelRendererCamera.mCameraPos",
  "LevelRendererCamera.mCameraTargetPos",
  "ViewRenderObject.mViewData",
  "ViewRenderData.mCameraPos",
  # head position and health (game.actor_extra)
  "Actor.getAttachPos",
  "Actor.mEntityContext",
  "ActorAttribute.getHealth",
  # delivery: the packet handler capture and the camera instruction
  "ClientNetworkHandler.vtable",
  "ClientNetworkHandler.handle.NetworkChunkPublisherUpdatePacket",
  "ClientNetworkHandler.handle.CameraPresetsPacket",
  "ClientNetworkHandler.handle.CameraInstructionPacket",
  "MinecraftPackets.createPacket",
  "CameraInstructionPacket.mCameraInstruction",
  "CameraInstructionPacket.size",
  "CameraInstruction.mSpline",
  "Level.getCameraPresets",
  "CameraPreset.size",
  # optional, markers: path markers as primitive shapes
  "ClientNetworkHandler.handle.PrimitiveShapesPacket",
  "PrimitiveShapesPacket.mShapes",
  "PrimitiveShapeDataPayload.size",
  # optional, camera watch: which camera the game shows
  "ClientInstance.getCameraRegistry",
  "CameraRegistry.mCameraEntities",
  "CameraRegistry.cameraEntityStride",
  "CameraComponent.typeId",
  "CameraComponent.sizeFactor",
  "CameraComponent.sizeShift",
  "ActiveCameraComponent.typeId",
  # optional, look-at: the block and actor ray
  "BlockSource.clip",
  "BlockSource.fetchEntities",
  "Actor.aabbShapeComponent",
]
