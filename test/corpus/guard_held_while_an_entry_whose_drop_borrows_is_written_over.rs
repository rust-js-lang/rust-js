// A guard a `let` holds while an entry is written over, of a `State` whose
// destructor borrows the same cell again: Rust's borrow is still held as
// the old entry drops, so the destructor's `try_borrow_mut()` fails, and
// the guard stays counted for JS's to fail too (#1815).
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct History<State> {
    entries: Vec<Rc<State>>,
    index: usize,
}

struct Loud {
    name: u32,
    stack: Weak<RefCell<History<Loud>>>,
}

impl Drop for Loud {
    fn drop(&mut self) {
        if let Some(stack) = self.stack.upgrade() {
            println!("drop {} borrowed {}", self.name, stack.try_borrow_mut().is_err());
        }
    }
}

fn recorder<State>(history: Rc<RefCell<History<State>>>) -> impl Fn(Rc<State>) {
    move |state| {
        let mut history = history.borrow_mut();
        let index = history.index;
        history.entries[index] = state;
    }
}

fn main() {
    let history: Rc<RefCell<History<Loud>>> = Rc::new(RefCell::new(History {
        entries: Vec::new(),
        index: 0,
    }));
    let weak = Rc::downgrade(&history);
    history.borrow_mut().entries.push(Rc::new(Loud {
        name: 1,
        stack: weak.clone(),
    }));
    let record = recorder(Rc::clone(&history));
    record(Rc::new(Loud {
        name: 2,
        stack: weak.clone(),
    }));
    record(Rc::new(Loud { name: 3, stack: weak }));
    println!("{}", history.borrow().entries[0].name);
}
