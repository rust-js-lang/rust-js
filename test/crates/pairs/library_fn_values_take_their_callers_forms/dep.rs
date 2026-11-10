//! A library's generic functions whose `&mut T` is generic code's box, or
//! whose `&mut W` of a `W` given a `&mut` to an object is a box of that
//! box, as another crate's function values: each takes its callers' forms
//! (#1324, #1322).
use std::io::Write;

pub fn view<T: ?Sized>(r: &mut T) -> &T {
    &*r
}

pub fn stamped<W: Write>(w: &mut W) -> &mut W {
    w.write_all(b"s").unwrap();
    w
}

pub fn put<T>(slot: &mut T, with: T) -> T {
    std::mem::replace(slot, with)
}
