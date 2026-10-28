// A tail the program wrote in each branch stays in each, as written, the
// user's decision (wl6): React's throwAndUnwindWorkLoop's `if (didFatal) {
// panicOnRootError(..); return; }` and its `catch`'s `else {
// panicOnRootError(..); return; }`, which merged left a `$catchUnwind`, are
// a `try` and its `catch` (ADR 0564, amended).
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static LOG: Cell<u32> = const { Cell::new(0) };
}

fn thrown(x: u32) -> bool {
    if x == 3 {
        panic!("three");
    }
    x > 5
}

fn on_root_error(x: u32) {
    LOG.set(LOG.get() * 10 + x);
}

fn unwind(x: u32, return_fiber: Option<u32>) {
    match panic::catch_unwind(AssertUnwindSafe(|| thrown(x))) {
        Ok(did_fatal) => {
            if did_fatal {
                on_root_error(x);
                return;
            }
        }
        Err(error) => {
            if let Some(fiber) = return_fiber {
                LOG.set(fiber);
                panic::resume_unwind(error);
            } else {
                on_root_error(x);
                return;
            }
        }
    }
    LOG.set(LOG.get() + 1);
}

fn main() {
    for x in [1, 3, 7] {
        unwind(x, None);
        println!("{}", LOG.get());
    }
    let caught = panic::catch_unwind(|| unwind(3, Some(9)));
    println!("{} {}", caught.is_err(), LOG.get());
}
