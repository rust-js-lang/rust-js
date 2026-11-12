// A `RefCell`'s `take` or `replace` while it's borrowed panics, and what it
// was given, `take`'s default made first or `replace`'s value, is dropped as
// the panic unwinds, as std's `replace(&self, t: T)` drops its parameter
// (#1587).
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Log(u32);

impl Default for Log {
    fn default() -> Self {
        println!("default");
        Log(0)
    }
}

impl Drop for Log {
    fn drop(&mut self) {
        println!("drop {} panicking={}", self.0, std::thread::panicking());
    }
}

fn main() {
    let log = RefCell::new(Log(1));
    let taken = catch_unwind(AssertUnwindSafe(|| {
        let _held = log.borrow();
        let _again = log.take();
    }));
    println!("take caught {}", taken.is_err());
    let replaced = catch_unwind(AssertUnwindSafe(|| {
        let _held = log.borrow();
        let _old = log.replace(Log(2));
    }));
    println!("replace caught {}", replaced.is_err());
    let old = log.replace(Log(3));
    println!("replaced {}", old.0);

    // Of an `Option`, which JS holds in the cell's `value`.
    let maybe = RefCell::new(Some(Log(4)));
    let optional = catch_unwind(AssertUnwindSafe(|| {
        let _held = maybe.borrow_mut();
        let _old = maybe.replace(Some(Log(5)));
    }));
    println!("optional caught {}", optional.is_err());
}
