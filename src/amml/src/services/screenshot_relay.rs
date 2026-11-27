use amm_core::structs::event::ModEvent;
use amml_api::{Logger, ShotId};
use freecam_core::services::FreecamController;

use crate::services::{LinkSession, ScreenshotPath};

// carries core's capture requests to the loader and its answers back: core counts shots with its own sequence, the
// loader names them by `ShotId`, and the relay holds the pair of the one shot in flight with the file name it chose
pub struct ScreenshotRelay {
    path: Option<ScreenshotPath>,
    // core's sequence, the loader's id and the file name of the shot in flight
    pending: Option<(u32, ShotId, String)>,
}

impl ScreenshotRelay {
    pub fn new(path: Option<ScreenshotPath>) -> Self {
        if path.is_none() {
            curia::warn!("screenshot.folder is not an absolute folder; screenshots fail");
        }
        Self { path, pending: None }
    }

    // a changed folder from config.hcl
    pub fn set_path(&mut self, path: Option<ScreenshotPath>) {
        self.path = path;
    }

    // core asked for capture `sequence`: a refusal, a missing folder or a lost name answers it as failed at once
    pub fn request(&mut self, sequence: u32, session: &LinkSession, controller: &mut FreecamController) {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let Some(path) = self.path.as_ref().and_then(|path| path.next(&stamp)) else {
            controller.on_capture_taken(sequence, false);
            return;
        };
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        match session.request_screenshot(&path) {
            Some(id) => self.pending = Some((sequence, id, name)),
            None => controller.on_capture_taken(sequence, false),
        }
    }

    // passes a screenshot answer for the shot in flight to core; true when `event` was a screenshot answer
    pub fn on_event(&mut self, event: &ModEvent, controller: &mut FreecamController, log: &Logger) -> bool {
        match *event {
            ModEvent::ScreenshotTaken { id, ok } => {
                if let Some((core, _, _)) = self.pending.as_ref().filter(|pending| pending.1 == id) {
                    controller.on_capture_taken(*core, ok);
                    if !ok {
                        self.pending = None;
                    }
                }
                true
            }
            ModEvent::ScreenshotWritten { id, ok } => {
                if let Some((core, _, name)) = self.pending.take_if(|pending| pending.1 == id) {
                    log.info("screenshot written", &[("ok", if ok { "1" } else { "0" }), ("file", &name)]);
                    controller.on_capture_written(core, ok, &name);
                }
                true
            }
            _ => false,
        }
    }
}
