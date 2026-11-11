//! A library's generic function that makes an `Rc` of what it's given and
//! reads its counts, which its own crate never gives a value with a
//! destructor: its consumer that gives one compiles its own (ADR 0914),
//! whose MIR, with no THIR, says the `Rc` owns a value whose drop runs and
//! that its counts are read, so it's counted (#1332). Each function's type
//! parameter is named apart, as one counted counts every one of its name.
use std::rc::{Rc, Weak};

pub fn counted<A>(held: A) -> (usize, usize) {
    let a = Rc::new(held);
    let b = Rc::clone(&a);
    let n = Rc::strong_count(&a);
    drop(b);
    (n, Rc::strong_count(&a))
}

/// Shares it and reads no count: the last `Rc` of it drops it.
pub fn shared<B>(held: B) -> usize {
    let a = Rc::new(held);
    let b = Rc::clone(&a);
    drop(a);
    println!("one left");
    drop(b);
    2
}

pub fn weakly<W>(held: W) -> (bool, bool) {
    let a = Rc::new(held);
    let w: Weak<W> = Rc::downgrade(&a);
    let alive = w.upgrade().is_some();
    drop(a);
    (alive, w.upgrade().is_some())
}

/// Of a `Copy` value the consumer changes in place, which it compiles its
/// own of as the library may copy it (ADR 0914): no drop counts its `Rc`,
/// only the count it reads.
pub fn counted_copy<C: Copy>(held: C) -> (usize, C) {
    let a = Rc::new(held);
    let b = Rc::clone(&a);
    let n = Rc::strong_count(&a);
    drop(b);
    (n + Rc::strong_count(&a), *a)
}
