//! The consumer counts the `Rc`s of its `Counted` the library made, and of
//! its `Plain` none: one is compiled here, the other the library's (#1234).
use std::rc::Rc;

#[derive(Debug)]
struct Counted(&'static str);

#[derive(Debug)]
struct Plain(u16);

pub fn main() {
    let a = dep::make(Counted("a"));
    let (b, c) = dep::shared(&a);
    println!("{} {:?}", Rc::strong_count(&a), b);
    drop(b);
    println!("{} {:?}", Rc::strong_count(&c), c);
    let p = dep::make(Plain(9));
    let (q, r) = dep::shared(&p);
    println!("{:?} {:?} {:?}", p, q, r);
}
