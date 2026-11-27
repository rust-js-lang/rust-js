//@ run-fail: RefCell already mutably borrowed
// `Rc::make_mut` of a `RefCell` mutably borrowed, shared, panics where
// std's `CloneToUninit` clones it, `clone/uninit.rs:20:33`, read from std's
// MIR of every std function a call is resolved to (`clonedAt`, #2509), not
// the caller's.
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut only = Rc::new(RefCell::new(1u8));
    *Rc::make_mut(&mut only).get_mut() += 1;
    println!("{only:?}");
    let mut shared = Rc::new(RefCell::new(2u8));
    let other = Rc::clone(&shared);
    *Rc::make_mut(&mut shared).get_mut() += 1;
    println!("{shared:?} {other:?}");
    let _held = other.borrow_mut();
    let mut again = Rc::clone(&other);
    println!("{:?}", Rc::make_mut(&mut again).try_borrow().is_ok());
}
