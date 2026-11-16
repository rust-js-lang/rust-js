// A guard a `let` holds while an entry is moved out into the function's
// result, which its caller drops later while a shared borrow of the cell
// is live: the guard ends as the function returns, before the value's
// drop, whose `try_borrow_mut()` then meets the caller's borrow, not the
// guard, and a later one meets none, as natively (#1815).
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Loud {
    name: u32,
    stack: Weak<RefCell<Vec<Rc<Loud>>>>,
}

impl Drop for Loud {
    fn drop(&mut self) {
        if let Some(stack) = self.stack.upgrade() {
            let free = stack.try_borrow_mut().is_ok();
            let read = stack.try_borrow().is_ok();
            println!("drop {} free {} read {}", self.name, free, read);
        }
    }
}

fn take(stack: &RefCell<Vec<Rc<Loud>>>) -> Option<Rc<Loud>> {
    let mut entries = stack.borrow_mut();
    let last = entries.pop();
    println!("taken, {} left", entries.len());
    last
}

fn main() {
    let stack = Rc::new(RefCell::new(Vec::new()));
    for name in 1..=3 {
        let loud = Rc::new(Loud {
            name,
            stack: Rc::downgrade(&stack),
        });
        stack.borrow_mut().push(loud);
    }
    let taken = take(&stack);
    let held = stack.borrow();
    println!("held {}", held.len());
    drop(taken);
    drop(held);
    let again = take(&stack);
    drop(again);
    println!("{}", stack.borrow().len());
}
