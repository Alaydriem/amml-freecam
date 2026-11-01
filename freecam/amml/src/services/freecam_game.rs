use amml_game::{CameraWriter, GameEntries};
use amml_msvc::MsvcHeap;

use crate::game::{CameraRegistry, InstructionLayout, LookLayout, PacketHandler, PresetList, ShapeLayout};
use crate::services::{CameraInstructions, CameraWatch, LookRay, ShapeDelivery};
use crate::types::FreecamFeature;

// the game access of one enable, built from the resolved entries: the camera write, the camera instructions, and the
// optional parts (markers, camera watch, look-at), each None when its feature is off
pub struct FreecamGame {
    pub entries: GameEntries,
    pub writer: CameraWriter,
    pub handler: PacketHandler,
    pub instructions: CameraInstructions,
    pub shapes: Option<ShapeDelivery>,
    pub watch: CameraWatch,
    pub look: Option<LookRay>,
}

impl FreecamGame {
    // None when a required part cannot be built (an entry is missing or a layout does not fit); `heap` is the game
    // heap the shape vector is allocated from, and without it the markers stay off
    pub fn new(entries: GameEntries, heap: Option<MsvcHeap>) -> Option<Self> {
        let writer = CameraWriter::new(&entries)?;
        let handler = PacketHandler::new(&entries)?;
        let instructions =
            CameraInstructions::new(InstructionLayout::new(&entries)?, handler, PresetList::new(&entries)?);
        let shapes = FreecamFeature::is_on(&entries, FreecamFeature::MARKERS)
            .then(|| Some(ShapeDelivery::new(ShapeLayout::new(&entries)?, handler, heap?)))
            .flatten();
        let watch = CameraWatch::new(
            FreecamFeature::is_on(&entries, FreecamFeature::CAMERA_WATCH)
                .then(|| CameraRegistry::new(&entries))
                .flatten(),
        );
        let look = FreecamFeature::is_on(&entries, FreecamFeature::LOOK_AT)
            .then(|| LookLayout::new(&entries).map(LookRay::new))
            .flatten();
        Some(Self { entries, writer, handler, instructions, shapes, watch, look })
    }

    // one line per optional part that the entries switched on but whose layout check refused it
    pub fn refused_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut refused = |feature: &str, built: bool| {
            if FreecamFeature::is_on(&self.entries, feature) && !built {
                lines.push(format!("{feature} is off (its entries resolved, but the layout check refused them)"));
            }
        };
        refused(FreecamFeature::MARKERS, self.shapes.is_some());
        refused(FreecamFeature::CAMERA_WATCH, self.watch.is_on());
        refused(FreecamFeature::LOOK_AT, self.look.is_some());
        lines
    }

    // disable: the packets go back to the game and the server camera flag goes
    pub fn reset(&self) {
        self.instructions.reset();
        self.watch.reset();
        if let Some(shapes) = &self.shapes {
            shapes.reset();
        }
    }
}
