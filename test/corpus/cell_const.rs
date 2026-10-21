// A `const` is a new value at each use, a `Cell` in one too: changing one
// use leaves the next as the constant says.
use std::cell::{Cell, RefCell};

const START: Cell<i32> = Cell::new(5);
const LIMIT: RefCell<Option<u32>> = RefCell::new(None);
const SLOTS: [Cell<u32>; 2] = [Cell::new(1), Cell::new(2)];

fn bump() -> i32 {
    let c = START;
    c.set(c.get() + 1);
    c.get()
}

fn main() {
    println!("{} {} {}", bump(), bump(), START.get());
    let limit = LIMIT;
    *limit.borrow_mut() = Some(3);
    println!("{:?} {:?}", limit.borrow(), LIMIT.borrow());
    let slots = SLOTS;
    slots[1].set(7);
    println!("{} {} {}", slots[0].get(), slots[1].get(), SLOTS[1].get());
}
