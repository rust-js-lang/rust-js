//! Generic code holding a value of an associated type, in a library with no
//! destructor of its own: its consumers' may have one.
pub trait Zone {
    type Offset;
    fn offset(&self) -> Self::Offset;
}

pub fn hold<Z: Zone>(zone: &Z) -> u32 {
    let _offset = zone.offset();
    println!("held");
    1
}
