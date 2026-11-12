//! A library's generic function the crate compiles itself, of MIR alone,
//! that makes an `Rc` of the library's own type and gives it to the
//! library's code, which counts it: what it calls names is reached, so the
//! `Rc` is counted as the library counts it.
use std::fmt::Display;
use std::rc::Rc;
pub struct Node(pub u32);
pub fn count_it(r: &Rc<Node>) -> usize {
    Rc::strong_count(r)
}
pub fn shown<T: Display>(t: T) -> String {
    let r = Rc::new(Node(7));
    let _two = Rc::clone(&r);
    format!("[{t}] {} {}", r.0, count_it(&r))
}
