//! The library's function returned past a guard's drop, which can't change
//! it: after the `finally`, as React's `startGestureTransition` returns
//! `noop` (ADR 0693, amended).
use dep::idle;
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
        println!("restored {}", self.previous);
        DEPTH.set(self.previous);
    }
}

fn ten() -> u32 {
    10
}

fn scope(n: u32) -> Option<u32> {
    if n == 3 {
        panic!("scope {n}");
    }
    (n > 1).then_some(n)
}

fn start(n: u32) -> &'static dyn Fn() -> u32 {
    let previous = DEPTH.get();
    DEPTH.set(previous + 1);
    let _restore = Restore { previous };
    match catch_unwind(AssertUnwindSafe(|| scope(n))) {
        Ok(Some(_)) => return &ten,
        Ok(None) => {}
        Err(_) => println!("caught"),
    }
    &dep::noop
}

/// As `start`, its last a library's function its `use` names.
fn restart(n: u32) -> &'static dyn Fn() -> u32 {
    let previous = DEPTH.get();
    DEPTH.set(previous + 1);
    let _restore = Restore { previous };
    match catch_unwind(AssertUnwindSafe(|| scope(n))) {
        Ok(Some(_)) => return &ten,
        Ok(None) => {}
        Err(_) => println!("caught"),
    }
    &idle
}

pub fn main() {
    for n in 1..4 {
        println!("{n} gave {} at depth {}", start(n)(), DEPTH.get());
        println!("{n} gave {} again at depth {}", restart(n)(), DEPTH.get());
    }
}
