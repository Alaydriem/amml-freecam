use crate::structs::markers::MarkerKind;
use crate::structs::math::Vec3;

// one engine-drawn shape in world coordinates
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MarkerShape {
    pub kind: MarkerKind,
    // start of a line or arrow, the minimum corner of a box
    pub location: Vec3,
    // end of a line or arrow, the size of a box
    pub end: Vec3,
    // ARGB
    pub color: u32,
}

impl MarkerShape {
    pub fn new(kind: MarkerKind, location: Vec3, end: Vec3, color: u32) -> Self {
        Self { kind, location, end, color }
    }
}
