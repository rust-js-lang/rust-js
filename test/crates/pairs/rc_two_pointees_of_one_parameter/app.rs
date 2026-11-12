//! The consumer counts its `u32`'s `Rc`, not its `Slot<u32>`'s (#1234).
use std::rc::Rc;

pub fn main() {
    let (r, n) = dep::both(4u32);
    let s = Rc::clone(&r);
    println!("{n} {} {}", Rc::strong_count(&r), dep::read(&s));
}
