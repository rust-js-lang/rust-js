//@ compile-fail: rust-js does not support this `&mut` given to `mem::swap` and the like, from its MIR yet
// A guard is the cell it guards in JS, so swapping two through `&mut`s of
// them would swap what the cells hold, where Rust swaps the guards: it's
// refused, where the guards aren't variables (#150). Two variables swap
// as variables do.
use std::cell::RefCell;

fn main() {
    let x = RefCell::new(1);
    let y = RefCell::new(2);
    std::mem::swap(&mut x.borrow_mut(), &mut y.borrow_mut());
    println!("{} {}", x.borrow(), y.borrow());
}
