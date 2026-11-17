// A `Drop` that makes a guard of its own, dropped where it's inlined, at
// the end of the scope and where a panic unwinds it, each its own guard:
// the inner guard's restore runs once on each way out (ADR 0608).
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restored {} (was {})", self.previous, DEPTH.get());
        DEPTH.set(self.previous);
    }
}

struct Outer(u32);

impl Drop for Outer {
    fn drop(&mut self) {
        let previous = DEPTH.replace(self.0 + 100);
        let _restore = Restore { previous };
        println!("outer {} at {}", self.0, DEPTH.get());
    }
}

fn run(n: u32) -> u32 {
    let _outer = Outer(n);
    if n > 5 {
        panic!("too big {n}");
    }
    n * 2
}

fn main() {
    for n in [2, 7] {
        match catch_unwind(AssertUnwindSafe(|| run(n))) {
            Ok(v) => println!("ok {v} at {}", DEPTH.get()),
            Err(_) => println!("caught at {}", DEPTH.get()),
        }
    }
}
