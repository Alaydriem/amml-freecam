#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CurveSample {
    // blocks travelled along the curve from its start
    pub distance: f32,
    pub segment: u32,
    // position inside the segment, 0 at its first point and 1 at its last
    pub t: f32,
}
