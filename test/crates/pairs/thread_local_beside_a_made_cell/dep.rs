//! A library whose function makes a cell of a reference,
//! `Cell::new(None)` of a `Cell<Option<&str>>`, and counts in a
//! thread-local of its own.

use std::cell::Cell;

pub struct Fiber {
    pub name: Cell<Option<&'static str>>,
    pub id: u32,
}

thread_local! {
    static NEXT_ID: Cell<u32> = const { Cell::new(0) };
}

pub fn create_fiber() -> Fiber {
    let id = NEXT_ID.get();
    NEXT_ID.set(id + 1);
    Fiber {
        name: Cell::new(None),
        id,
    }
}
