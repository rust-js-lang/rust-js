//@ run-fail: RefCell already mutably borrowed
// `Rc::unwrap_or_clone` of a `RefCell` mutably borrowed, shared, panics
// where std's clones it, `rc.rs:2250:56`, read from std's MIR of every std
// function a call is resolved to (`clonedAt`, #2509), not the caller's.
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let only = Rc::new(RefCell::new(1u8));
    println!("{:?}", Rc::unwrap_or_clone(only));
    let shared = Rc::new(RefCell::new(2u8));
    let other = Rc::clone(&shared);
    println!("{:?}", Rc::unwrap_or_clone(shared));
    let shared = Rc::new(RefCell::new(3u8));
    let _held = other.borrow_mut();
    let kept = Rc::clone(&shared);
    let _held_too = kept.borrow_mut();
    println!("{:?}", Rc::unwrap_or_clone(shared));
}
