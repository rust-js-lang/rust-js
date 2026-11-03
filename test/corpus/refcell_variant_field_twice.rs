//@ run-fail: RefCell already borrowed
// An enum variant's `RefCell` field borrowed through a pattern's binding
// while it's borrowed already finds it borrowed, as std's does (ADR 0863,
// amended, #503): the binding is the field's own place, its borrows
// counted where one may meet another.
use std::cell::RefCell;

pub enum Kind {
    Effect { cleanup: RefCell<Option<u32>> },
    Scope,
}

fn main() {
    let kinds = [
        Kind::Effect {
            cleanup: RefCell::new(Some(1)),
        },
        Kind::Scope,
    ];
    for kind in &kinds {
        if let Kind::Effect { cleanup } = kind {
            *cleanup.borrow_mut() = Some(2);
            let held = cleanup.borrow();
            println!("{:?}", *held);
            *cleanup.borrow_mut() = None;
            println!("{:?}", *held);
        }
    }
}
