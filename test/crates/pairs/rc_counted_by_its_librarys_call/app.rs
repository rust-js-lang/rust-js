//! The consumer counts no `Rc` of its `i8`, and gives the library's
//! generic `peek`, whose JS counts, one: compiled here (#1234).
use std::rc::Rc;

pub fn main() {
    let r = Rc::new(-3i8);
    let s = Rc::clone(&r);
    println!("{} {}", dep::peek(&s), dep::peek(&r));
    println!("{}", dep::own_count());
}
