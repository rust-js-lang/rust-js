//@ run-pass
// A generic enum's variant's `RefCell<T>` field, made of its value where
// the variant is made, and read through a pattern's binding, is that
// field's own, no `RefCell<T>` of any value that generic code is given
// (ADR 0863, amended, #503): a `RefCell<Vec<i32>>` elsewhere, lent and
// borrowed, is the array.
use std::cell::RefCell;

pub enum Slot<T> {
    Full { current: RefCell<T> },
    Empty,
}

pub fn full<T>(value: T) -> Slot<T> {
    Slot::Full {
        current: RefCell::new(value),
    }
}

pub fn put<T>(slot: &Slot<T>, value: T) -> Option<T> {
    match slot {
        Slot::Full { current } => Some(current.replace(value)),
        Slot::Empty => None,
    }
}

fn total(log: &RefCell<Vec<i32>>) -> i32 {
    log.borrow().iter().sum()
}

fn main() {
    let slot = full(vec![1, 2]);
    if let Slot::Full { current } = &slot {
        current.borrow_mut().push(3);
    }
    let log = RefCell::new(vec![4, 5]);
    log.borrow_mut().push(6);
    let old = put(&slot, vec![7]);
    if let Slot::Full { current } = &slot {
        println!("{old:?} {:?} {}", current.borrow(), total(&log));
    }
    println!("{:?}", put(&Slot::Empty, 8));
}
