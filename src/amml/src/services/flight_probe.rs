use amm_core::structs::event::KeybindPhase;
use amml_api::Logger;
use freecam_core::structs::{FreecamAction, Vec3};

use crate::types::InputItem;

// the log lines that show a flight working in a run: how far the camera moved while flight keys were held, a new
// path point, and the flight keys cleared by a focus loss. Each line comes from an event a person caused (a key
// released, a point added, a focus lost), so the count of lines stays small
pub struct FlightProbe {
    // the flight binds held now
    held: u8,
    // the camera position when the first flight bind went down
    start: Option<Vec3>,
    point_count: usize,
}

impl FlightProbe {
    pub fn new() -> Self {
        Self { held: 0, start: None, point_count: 0 }
    }

    // the binds of this frame and the camera position of this frame (None while freecam does not fly)
    pub fn inputs(&mut self, inputs: &[InputItem], position: Option<Vec3>, log: &Logger) {
        for input in inputs {
            let InputItem::Bind(action, phase) = *input else { continue };
            if !Self::is_flight(action) {
                continue;
            }
            match phase {
                KeybindPhase::Pressed => {
                    if self.held == 0 {
                        self.start = position;
                    }
                    self.held = self.held.saturating_add(1);
                }
                KeybindPhase::Released => {
                    self.held = self.held.saturating_sub(1);
                    if self.held == 0 {
                        self.moved(position, log);
                    }
                }
                KeybindPhase::Repeat => {}
            }
        }
    }

    // the path's point count after this frame
    pub fn points(&mut self, count: usize, log: &Logger) {
        if count > self.point_count {
            log.info("path point added", &[("count", &count.to_string())]);
        }
        self.point_count = count;
    }

    // a frame that follows a focus loss, while freecam flies, cleared the held flight keys
    pub fn focus(&mut self, focused: bool, flying: bool, log: &Logger) {
        if !focused && flying {
            log.info("flight keys cleared", &[("reason", "focus_lost")]);
        }
    }

    fn moved(&mut self, position: Option<Vec3>, log: &Logger) {
        let (Some(start), Some(end)) = (self.start.take(), position) else { return };
        let distance = (end - start).length();
        log.info("flight moved", &[("distance", &format!("{distance:.2}"))]);
    }

    fn is_flight(action: FreecamAction) -> bool {
        matches!(
            action,
            FreecamAction::FlyForward
                | FreecamAction::FlyBack
                | FreecamAction::FlyLeft
                | FreecamAction::FlyRight
                | FreecamAction::FlyUp
                | FreecamAction::FlyDown
        )
    }
}

impl Default for FlightProbe {
    fn default() -> Self {
        Self::new()
    }
}
