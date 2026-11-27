use amml_api::{CameraKind, InstructionKind};

// whether a player camera is the one the game shows. Two sources: the active camera the loader reads (the camera
// watch feature) and the server's camera instructions (a set means a server camera is on, a clear that the player
// camera is back). The active camera wins when the loader can read it; without it the instructions decide. A change
// of the active camera is kept for one log line
pub struct CameraWatch {
    server_camera: bool,
    // the last active camera read, `None` inside for a frame without a read; the outer `None` before the first frame
    last: Option<Option<CameraKind>>,
    changed: Option<&'static str>,
}

impl CameraWatch {
    pub fn new() -> Self {
        Self { server_camera: false, last: None, changed: None }
    }

    // a camera instruction from the server reached the client (the mod's own are not passed here)
    pub fn on_server_instruction(&mut self, kind: InstructionKind) {
        match kind {
            InstructionKind::Set => self.server_camera = true,
            InstructionKind::Clear => self.server_camera = false,
            _ => {}
        }
    }

    // the decision of one frame from the active camera read (None: no read)
    pub fn player_camera_active(&mut self, kind: Option<CameraKind>) -> bool {
        if self.last != Some(kind) {
            self.last = Some(kind);
            self.changed = Some(Self::name(kind));
        }
        match kind {
            Some(CameraKind::FirstPerson | CameraKind::ThirdPerson | CameraKind::ThirdPersonFront) => true,
            Some(_) => false,
            None => !self.server_camera,
        }
    }

    // the name of the active camera when it changed since the last call
    pub fn take_changed(&mut self) -> Option<&'static str> {
        self.changed.take()
    }

    // a world leave: the server camera goes with the world
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn name(kind: Option<CameraKind>) -> &'static str {
        match kind {
            Some(CameraKind::FirstPerson) => "first_person",
            Some(CameraKind::ThirdPerson) => "third_person",
            Some(CameraKind::ThirdPersonFront) => "third_person_front",
            Some(_) => "other",
            None => "unknown",
        }
    }
}

impl Default for CameraWatch {
    fn default() -> Self {
        Self::new()
    }
}
