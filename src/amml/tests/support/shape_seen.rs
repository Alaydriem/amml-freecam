// one payload of a PrimitiveShapesPacket the fake handler got
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeSeen {
    pub id: u64,
    // the shape type, None when its option is disengaged (a remove)
    pub shape_type: Option<u8>,
    pub extra_index: u8,
}
