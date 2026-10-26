// React's `runWithFiberInDEV` (ReactCurrentFiber.js) sets the current
// fiber, runs the callback, and restores the previous one in a `finally`,
// however it's left: `try { if (..) { return ..; } return callback(..); }
// finally { setCurrentFiber(previousFiber); }`. Its port restores it in a
// guard's `Drop`, which takes what it holds, and returns early from the
// guard's scope; each way out, a panic too, restores it once.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn set_current(name: Option<String>) {
    LOG.with_borrow_mut(|log| log.push(format!("set {name:?}")));
    CURRENT.set(name);
}

fn current() -> Option<String> {
    CURRENT.with_borrow(|current| current.clone())
}

struct Restore(Option<String>);

impl Drop for Restore {
    fn drop(&mut self) {
        set_current(self.0.take());
    }
}

fn run_with<T>(name: &str, wrap: bool, callback: impl FnOnce() -> T) -> T {
    let previous = current();
    set_current(Some(name.to_string()));
    let _restore = Restore(previous);
    if wrap {
        return callback();
    }
    callback()
}

fn main() {
    let outer = run_with("outer", false, || {
        let inner = run_with("inner", true, || current().unwrap_or_default().len());
        format!("{inner} {:?}", current())
    });
    println!("{outer} {:?}", current());
    let caught = catch_unwind(AssertUnwindSafe(|| {
        run_with("panics", true, || -> u32 { panic!("thrown in {:?}", current()) })
    }));
    println!("{} {:?}", caught.is_err(), current());
    for line in LOG.with_borrow(|log| log.clone()) {
        println!("{line}");
    }
}
