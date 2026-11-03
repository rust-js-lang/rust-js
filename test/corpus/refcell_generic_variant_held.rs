//@ run-fail: RefCell already borrowed
// A generic enum's variant's `RefCell<T>` field holds its value; made of an
// array and borrowed in its generic code while code that may borrow it
// runs, the `RefCell<Vec<u8>>` its concrete code borrows is a box of it
// too, so both count on the field's one cell, and the mutable borrow
// panics as std's does (ADR 0863, amended, #503).
use std::cell::RefCell;

enum Slot<T> {
    Full { current: RefCell<T> },
    Empty,
}

fn after<T: Clone>(value: &T, f: impl Fn()) -> T {
    f();
    value.clone()
}

fn peek<T: Clone>(slot: &Slot<T>, f: impl Fn()) -> Option<T> {
    match slot {
        Slot::Full { current } => Some(after(&*current.borrow(), f)),
        Slot::Empty => None,
    }
}

fn main() {
    let slot = Slot::Full {
        current: RefCell::new(vec![1u8]),
    };
    println!("{:?} {:?}", peek(&slot, || {}), peek(&Slot::<u8>::Empty, || {}));
    let seen = peek(&slot, || {
        if let Slot::Full { current } = &slot {
            current.borrow_mut().push(2);
        }
    });
    println!("{seen:?}");
}
