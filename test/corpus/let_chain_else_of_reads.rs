// A let chain of two `let`s with an `else`, each binding a read of a
// thread-local, is one `if` of both tests and its `else`: `if (lanes !=
// null && root != null) { .. } else { throw .. }`, as React's
// attachSuspenseRetryListeners writes it, not tests that break a labeled
// block (ADR 0946, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Root {
    lanes: Cell<u32>,
}

thread_local! {
    static IN_PROGRESS_LANES: Cell<Option<u32>> = const { Cell::new(None) };
    static IN_PROGRESS_ROOT: RefCell<Option<Rc<Root>>> = const { RefCell::new(None) };
}

fn restore() -> u32 {
    if let Some(in_progress_lanes) = IN_PROGRESS_LANES.get()
        && let Some(in_progress_root) = IN_PROGRESS_ROOT.with_borrow(Clone::clone)
    {
        in_progress_root.lanes.set(in_progress_lanes);
        in_progress_root.lanes.get() * 2
    } else {
        panic!("Expected finished root and lanes to be set. This is a bug in React.");
    }
}

fn main() {
    let root = Rc::new(Root { lanes: Cell::new(0) });
    let caught = std::panic::catch_unwind(restore);
    println!("{}", caught.is_err());
    IN_PROGRESS_LANES.set(Some(3));
    println!("{}", std::panic::catch_unwind(restore).is_err());
    IN_PROGRESS_ROOT.set(Some(Rc::clone(&root)));
    println!("{} {}", restore(), root.lanes.get());
}
