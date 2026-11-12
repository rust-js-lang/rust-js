//! The consumer counts its `u32`s' `Rc`s, reading one's count, and gives
//! its library's generic functions one: each compiled here (#1234).
use std::rc::Rc;

pub fn main() {
    let a = Rc::new(7u32);
    let b = Rc::clone(&a);
    println!("{} {}", Rc::strong_count(&a), dep::get(&b));
    let (c, d) = dep::twice(&a);
    println!("{} {} {}", Rc::strong_count(&a), c, d);
    drop((b, c, d));
    println!("{}", Rc::strong_count(&a));
    let later = dep::make_later(9u32);
    println!("{}", later());
}
