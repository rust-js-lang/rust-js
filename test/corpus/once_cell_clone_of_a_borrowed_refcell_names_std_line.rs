//@ run-fail: RefCell already mutably borrowed
// A `OnceCell`'s clone of a `RefCell` mutably borrowed panics where std's
// `OnceCell` clone clones it, `once.rs:406:49`, read from std's MIR
// (`clonedAt`), never the line of the `Option` it's held as, which std's
// clone doesn't call (#2509).
use std::cell::{OnceCell, RefCell};

fn main() {
    let empty: OnceCell<RefCell<u8>> = OnceCell::new();
    println!("{:?}", empty.clone().get());
    let cell: OnceCell<RefCell<u8>> = OnceCell::new();
    cell.set(RefCell::new(1)).unwrap();
    println!("{:?}", cell.clone().get());
    let _held = cell.get().unwrap().borrow_mut();
    println!("{:?}", cell.clone().get().is_some());
}
