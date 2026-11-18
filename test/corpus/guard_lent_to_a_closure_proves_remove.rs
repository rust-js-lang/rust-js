// A `RefCell` guard's fields are locals of their own to MIR's intervals
// (#1815), though a closure is given `&guard`, a shared reference through
// which nothing but `&History` is reached: `remove(0)` past a push of a
// longer `Vec` is proved in bounds, as the undo stack's `record` (#1944).
// A field with interior mutability, a `Cell` the closure changes through
// it, is known of nothing, and its index keeps its check.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct History {
    entries: Vec<u32>,
    open: Option<u32>,
    at: Cell<usize>,
}

fn record(history: &Rc<RefCell<History>>, entry: u32, limit: usize) -> bool {
    let mut history = history.borrow_mut();
    let joins = Some(entry).is_some_and(|entry| history.open.is_some_and(|open| open == entry));
    history.entries.push(entry);
    if history.entries.len() > limit + 1 {
        history.entries.remove(0);
    }
    history.open = Some(entry);
    joins
}

fn moved(history: &Rc<RefCell<History>>, entry: u32) -> u32 {
    let mut history = history.borrow_mut();
    history.entries.push(entry);
    let step = |by: usize| {
        history.at.set(history.at.get() + by);
        history.at.get()
    };
    let at = step(3);
    history.entries[at]
}

fn main() {
    let history = Rc::new(RefCell::new(History {
        entries: vec![1, 2],
        open: None,
        at: Cell::new(0),
    }));
    for entry in [3, 3, 4, 5] {
        println!("{}", record(&history, entry, 2));
    }
    println!("{:?}", history.borrow().entries);
    println!("{}", moved(&history, 6));
    let reached = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| moved(&history, 7)));
    println!("{}", reached.is_err());
}
