//! A library's generic function that shares an `Rc` of what it's given and
//! reads no count of it: its consumer that gives a value with a destructor
//! compiles its own (ADR 0914), whose MIR's locals own that `Rc`, so it's
//! counted and the last of them drops the value (#1332).
use std::rc::Rc;

pub fn shared<B>(held: B) -> usize {
    let a = Rc::new(held);
    let b = Rc::clone(&a);
    drop(a);
    println!("one left");
    drop(b);
    2
}
