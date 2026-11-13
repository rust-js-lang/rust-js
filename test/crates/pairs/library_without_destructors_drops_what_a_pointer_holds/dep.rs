//! A library with no destructor of its own and no library under it, whose
//! generic code holds a value of an associated type, reached through a
//! function pointer it gives out: its consumer can't compile its own of what
//! it reaches so, so the library's own drops what its consumer's type has a
//! destructor for (ADR 0163).
pub trait Zone {
    type Offset;
    fn offset(&self) -> Self::Offset;
}

pub fn hold<Z: Zone>(zone: &Z) -> u32 {
    let _offset = zone.offset();
    println!("held");
    1
}

pub fn holder<Z: Zone>() -> fn(&Z) -> u32 {
    hold::<Z>
}
