//! Two kinds of `Rc<dyn Fn()>` a library hands its consumer, of one
//! argument list and two results (#1056): one whose function a cloned
//! closure merges, which a consumer that compares them compiles and counts,
//! and one the library counts none of.
use std::rc::Rc;

/// Two `Rc`s of one closure, cloned: two allocations.
pub fn reads(n: u32) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let read = move || n;
    (Rc::new(read.clone()), Rc::new(read))
}

/// A listener, of no result.
pub fn listener(name: &'static str) -> Rc<dyn Fn()> {
    Rc::new(move || println!("heard {name}"))
}
