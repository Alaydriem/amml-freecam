// what a stored camera pose does to the path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEdit {
    // goes in at the playback bar's insert point
    Insert,
    // takes the place of the key at this index
    Replace(usize),
}
