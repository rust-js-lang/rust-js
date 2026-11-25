//@ run-fail: RefCell already mutably borrowed
// `write_clone_of_slice` of a `RefCell` mutably borrowed panics where
// std clones each item into its slot, `mem/maybe_uninit.rs`, never its
// caller's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let items = [RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let mut slots = [const { std::mem::MaybeUninit::<RefCell<u8>>::uninit() }; 2];
    println!("{}", slots.len());
    let written = slots.write_clone_of_slice(&items);
    println!("{}", written.len());
}
