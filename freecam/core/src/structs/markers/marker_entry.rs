use crate::structs::markers::MarkerShape;

// one slot change for the client: draw or move the shape under `id`, or remove it when the kind is `Remove`
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MarkerEntry {
    pub id: u64,
    pub shape: MarkerShape,
    pub dimension: i32,
}

impl MarkerEntry {
    pub fn new(id: u64, shape: MarkerShape, dimension: i32) -> Self {
        Self { id, shape, dimension }
    }
}
