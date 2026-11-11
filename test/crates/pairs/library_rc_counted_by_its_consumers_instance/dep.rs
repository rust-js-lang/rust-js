//! A library's generic function that reads the counts of an `Rc` of a
//! `Copy` value its consumer changes in place, which the consumer compiles
//! its own of, as the library may copy it (ADR 0914): no drop counts the
//! `Rc`, only the counts its MIR reads (#1332).
use std::rc::Rc;

pub fn counted_copy<C: Copy>(held: C) -> (usize, C) {
    let a = Rc::new(held);
    let b = Rc::clone(&a);
    let n = Rc::strong_count(&a);
    drop(b);
    (n + Rc::strong_count(&a), *a)
}
