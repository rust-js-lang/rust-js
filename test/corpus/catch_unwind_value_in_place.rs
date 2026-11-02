// A `try`'s value the program never named, read by the `Result`'s name, is
// the call where the `try` reads it first: React's
// isRenderConsistentWithExternalStores, `if (!objectIs(getSnapshot(),
// renderedValue)) { return false; }` (ADR 0706, amended); one the program
// names, read twice, keeps its name. And a closure that returns early, its
// value a `let`'s, sets it in each branch, what follows a guard its `else`
// (rd60): a guard, an `if` whose branches each return, one that panics,
// and one whose `return` is in a loop, which stays a closure called.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static LOG: Cell<u32> = const { Cell::new(0) };
}

fn note(n: u32) {
    LOG.set(LOG.get() * 10 + n % 10);
}

fn step(n: u32) -> u32 {
    note(n);
    if n > 5 {
        panic!("big {n}");
    }
    n + 1
}

fn same(a: u32, b: u32) -> bool {
    if a > 7 {
        panic!("same {a}");
    }
    a == b
}

/// isRenderConsistentWithExternalStores's.
fn consistent(checks: &[(u32, u32)]) -> bool {
    let mut i = 0;
    while i < checks.len() {
        let (a, b) = checks[i];
        match catch_unwind(AssertUnwindSafe(|| same(a, b))) {
            Ok(true) => {}
            Ok(false) => {
                // Found an inconsistent store.
                return false;
            }
            Err(_) => {
                // If it throws, return `false`.
                return false;
            }
        }
        i += 1;
    }
    true
}

/// The value tested where it's returned, a comment before it.
fn picked(n: u32) -> u32 {
    match catch_unwind(|| {
        let a = step(n);
        // Whether it's three.
        same(a, 3)
    }) {
        Ok(true) => 1,
        Ok(false) => 2,
        Err(_) => 3,
    }
}

/// A value the program names, read twice.
fn twice(n: u32) -> u32 {
    match catch_unwind(|| same(n, 3)) {
        Ok(found) if found => 1,
        Ok(found) => u32::from(found) + 2,
        Err(_) => 3,
    }
}

/// A value the program names, read once.
fn named_once(n: u32) -> u32 {
    match catch_unwind(|| same(n, 4)) {
        Ok(found) => {
            if found {
                5
            } else {
                6
            }
        }
        Err(_) => {
            note(8);
            7
        }
    }
}

/// A guard, in an outer `catch_unwind`'s closure.
fn guarded(n: u32) -> u32 {
    match catch_unwind(|| {
        let found = match catch_unwind(|| {
            if n == 0 {
                return 0;
            }
            let a = step(n);
            a * 2
        }) {
            Ok(v) => v,
            Err(_) => return 3,
        };
        step(found)
    }) {
        Ok(v) => v,
        Err(_) => {
            note(9);
            9
        }
    }
}

/// An `if` whose branches each return, one that panics, then the rest.
fn branches(n: u32) -> u32 {
    let found = match catch_unwind(|| {
        if n % 2 != 1 {
            if n > 4 {
                panic!("even {n}");
            }
            return n / 2;
        } else if n == 1 {
            return 40;
        }
        let a = step(n);
        a + 10
    }) {
        Ok(v) => v,
        Err(_) => {
            note(5);
            50
        }
    };
    found + 1
}

/// A `return` in a loop.
fn looped(n: u32) -> u32 {
    let found = match catch_unwind(|| {
        for i in 0..n {
            if step(i) > 3 {
                return i;
            }
        }
        n
    }) {
        Ok(v) => v,
        Err(_) => {
            note(7);
            70
        }
    };
    found + 1
}

fn main() {
    println!(
        "consistent {} {} {}",
        consistent(&[(1, 1), (2, 2)]),
        consistent(&[(1, 1), (2, 3), (4, 4)]),
        consistent(&[(9, 9), (1, 1)])
    );
    for n in [0, 1, 2, 3, 4, 6, 9] {
        LOG.set(0);
        let values = [picked(n), twice(n), named_once(n), guarded(n), branches(n), looped(n)];
        println!("{n} {values:?} {}", LOG.get());
    }
}
