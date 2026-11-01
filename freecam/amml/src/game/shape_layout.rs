use std::ffi::c_void;
use std::mem::offset_of;

use amml_game::GameEntries;
use amml_game::game::{ArrowDataLayout, PrimitiveShapeLayout};
use amml_msvc::{Optional, SharedPtr};
use freecam_core::structs::{MarkerEntry, MarkerKind};

use crate::game::PacketFactory;
use crate::types::FreecamEntry;

// the PrimitiveShapesPacket and its payloads: how to make one, where its shape vector sits, and each payload written
// through the PrimitiveShapeLayout mirror (checked by `PrimitiveShapeDataPayload.size`)
#[derive(Debug, Clone, Copy)]
pub struct ShapeLayout {
    factory: PacketFactory,
    shapes: usize,
}

impl ShapeLayout {
    // ScriptModuleMinecraft::ScriptPrimitiveShapeType (the script API enum)
    pub const SHAPE_LINE: u8 = 0;
    pub const SHAPE_BOX: u8 = 1;
    pub const SHAPE_ARROW: u8 = 5;
    // the alternative index of the extra data variant: NullType, Arrow, Text, Box, Line, Sphere, ...
    pub const EXTRA_ARROW: u8 = 1;
    pub const EXTRA_BOX: u8 = 3;
    pub const EXTRA_LINE: u8 = 4;
    // the arrow head that drew in the shape spike
    pub const ARROW_HEAD_LENGTH: f32 = 0.4;
    pub const ARROW_HEAD_RADIUS: f32 = 0.2;
    pub const ARROW_HEAD_SEGMENTS: u8 = 8;
    // the bytes of one payload
    pub const SIZE: usize = size_of::<PrimitiveShapeLayout>();

    pub fn new(entries: &GameEntries) -> Option<Self> {
        let get = |id| usize::try_from(entries.get(id)?).ok();
        let fits = get(FreecamEntry::PAYLOAD_SIZE)? == Self::SIZE;
        fits.then(|| {
            Some(Self {
                factory: PacketFactory::new(get(FreecamEntry::CREATE_PACKET)?),
                shapes: get(FreecamEntry::SHAPES_PACKET_SHAPES)?,
            })
        })
        .flatten()
    }

    // a new PrimitiveShapesPacket from the game's factory; None when the game returns none or another packet
    pub fn create_packet(&self) -> Option<SharedPtr<c_void>> {
        self.factory.create(PacketFactory::PRIMITIVE_SHAPES_ID, PacketFactory::PRIMITIVE_SHAPES_NAME)
    }

    // the address of the shape vector (MSVC: first, last, end) in `packet`
    pub fn vector(&self, packet: *mut c_void) -> *mut [*mut u8; 3] {
        packet.cast::<u8>().wrapping_add(self.shapes).cast()
    }

    /// writes one marker into the zeroed payload at `payload`: a remove carries the id only; a line, a box or an arrow
    /// gets its type, location, no rotation, scale 1, color, dimension and its extra data
    ///
    /// # Safety
    /// `payload` must point to `SIZE` zeroed, writable bytes
    #[allow(unsafe_code)]
    pub unsafe fn fill(&self, payload: *mut u8, entry: &MarkerEntry) {
        let shape = payload.cast::<PrimitiveShapeLayout>();
        let (kind, extra) = match entry.shape.kind {
            MarkerKind::Remove => (None, 0),
            MarkerKind::Line => (Some(Self::SHAPE_LINE), Self::EXTRA_LINE),
            MarkerKind::Box => (Some(Self::SHAPE_BOX), Self::EXTRA_BOX),
            MarkerKind::Arrow => (Some(Self::SHAPE_ARROW), Self::EXTRA_ARROW),
        };
        let location = entry.shape.location;
        let end = [entry.shape.end.x, entry.shape.end.y, entry.shape.end.z];
        // SAFETY: the caller guarantees `SIZE` zeroed, writable bytes, laid out as the mirror; every member is
        // trivially copyable. The payload buffer is only byte aligned, so each member is written unaligned
        unsafe {
            (&raw mut (*shape).network_id).write_unaligned(entry.id);
            let Some(kind) = kind else { return };
            (&raw mut (*shape).shape_type).write_unaligned(Optional::some(kind));
            (&raw mut (*shape).location).write_unaligned(Optional::some([location.x, location.y, location.z]));
            (&raw mut (*shape).rotation).write_unaligned(Optional::some([0.0; 3]));
            (&raw mut (*shape).scale).write_unaligned(Optional::some(1.0));
            (&raw mut (*shape).color).write_unaligned(Optional::some(Self::color(entry.shape.color)));
            (&raw mut (*shape).dimension).write_unaligned(Optional::some(entry.dimension));
            (&raw mut (*shape).extra_index).write_unaligned(extra);
            let data = payload.add(offset_of!(PrimitiveShapeLayout, extra));
            match entry.shape.kind {
                MarkerKind::Arrow => data.cast::<ArrowDataLayout>().write_unaligned(ArrowDataLayout {
                    end: Optional::some(end),
                    head_length: Optional::some(Self::ARROW_HEAD_LENGTH),
                    head_radius: Optional::some(Self::ARROW_HEAD_RADIUS),
                    segments: Optional::some(Self::ARROW_HEAD_SEGMENTS),
                }),
                _ => data.cast::<[f32; 3]>().write_unaligned(end),
            }
        }
    }

    // mce::Color from ARGB: r, g, b, a as floats in 0..1
    pub fn color(argb: u32) -> [f32; 4] {
        let channel = |shift: u32| ((argb >> shift) & 0xFF) as f32 / 255.0;
        [channel(16), channel(8), channel(0), channel(24)]
    }
}
