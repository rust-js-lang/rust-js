// A flag a closure gives out of `catch_unwind` to return from its function,
// which a closure can't, as React's dispatchSetStateInternal returns from
// inside its `try`: `try { ..; if (c) { ..; return false; } } catch {}`,
// what it caught dropped, as JS writes it (ADR 0660, amended).

use std::panic::{self, AssertUnwindSafe};

fn bailout(log: &mut Vec<String>) {
    log.push("bailout".to_string());
}

fn dispatch(reducer: &dyn Fn(u32) -> u32, state: u32, log: &mut Vec<String>) -> bool {
    let did_bail_out = panic::catch_unwind(AssertUnwindSafe(|| {
        let eager_state = reducer(state);
        if eager_state == state {
            bailout(log);
            return true;
        }
        false
    }));
    if let Ok(true) = did_bail_out {
        return false;
    }
    log.push("scheduled".to_string());
    true
}

// Of the flag the other way, `!r._0`.
fn kept(reducer: &dyn Fn(u32) -> u32, state: u32) -> u32 {
    let changed = panic::catch_unwind(AssertUnwindSafe(|| {
        if reducer(state) != state {
            return false;
        }
        true
    }));
    if let Ok(false) = changed {
        return 0;
    }
    state
}

fn finish(state: u32) -> u32 {
    if state == 1 {
        panic!("finish threw");
    }
    state
}

// One whose `return` may throw stays the flag: in a `try`, what it threw
// would be caught.
fn finished(reducer: &dyn Fn(u32) -> u32, state: u32) -> u32 {
    let same = panic::catch_unwind(AssertUnwindSafe(|| {
        if reducer(state) == state {
            return true;
        }
        false
    }));
    if let Ok(true) = same {
        return finish(state);
    }
    7
}

// One whose `Result` is read after stays the flag.
fn caught(reducer: &dyn Fn(u32) -> u32, state: u32) -> bool {
    let same = panic::catch_unwind(AssertUnwindSafe(|| {
        if reducer(state) == state {
            return true;
        }
        false
    }));
    if let Ok(true) = same {
        return false;
    }
    same.is_err()
}

fn main() {
    println!("{} {}", caught(&|n| n + 1, 1), caught(&|_| panic!("caught threw"), 1));
    println!("{} {} {}", kept(&|n| n, 1), kept(&|n| n + 1, 1), kept(&|_| panic!("kept threw"), 2));
    let outer = panic::catch_unwind(|| finished(&|n| n, 1));
    println!("{} {}", outer.is_err(), finished(&|n| n + 1, 1));
    let mut log = Vec::new();
    let same = dispatch(&|n| n, 1, &mut log);
    let other = dispatch(&|n| n + 1, 1, &mut log);
    let thrown = dispatch(&|_| panic!("reducer threw"), 1, &mut log);
    println!("{same} {other} {thrown} {log:?}");
}
