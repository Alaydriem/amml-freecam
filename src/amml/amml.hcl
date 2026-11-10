namespace   = "alaydriem"
name        = "freecam"
entry       = "alaydriem_freecam.dll"
api_version = 2
amml_version = "26.60.6"
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

listing {
  title       = "Freecam"
  summary     = "Free camera with keyframed camera paths for recording smooth shots."
  description = <<-EOT
    A free camera mod to look at builds and scout out areas independently of your player position. It also has basic cinematic tools: create simple keyframe paths and quickly take screenshots.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-freecam"
  homepage    = "https://github.com/alaydriem/amml-freecam#readme"
  tags        = ["freecam", "camera", "cinematic", "recording", "client"]
  icon        = "icon.png"
}
