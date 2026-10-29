// A `catch_unwind` tested after what an `if`'s `&&` asks first is a `try`
// inside that `if` (ADR 0706, amended): React's checkPassiveEvents, `if
// (canUseDOM) { try { .. } catch (e) { supported = false; } }`. The closure
// runs only where what's first is true, and the rest of the chain is the
// `catch`'s `if`, each as Rust runs them.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static SUPPORTED: Cell<bool> = const { Cell::new(false) };
}

fn probe(n: u32) {
    SUPPORTED.set(true);
    if n > 2 {
        panic!("no passive {n}");
    }
}

/// checkPassiveEvents: what's asked first, then whether the closure threw.
fn check(can_use_dom: bool, n: u32) -> bool {
    SUPPORTED.set(false);
    if can_use_dom && catch_unwind(AssertUnwindSafe(|| probe(n))).is_err() {
        SUPPORTED.set(false);
    }
    SUPPORTED.get()
}

/// Two asked first, and one after it, which reads what the closure set.
fn counted(first: bool, second: bool, limit: u32) -> u32 {
    let mut seen = 0;
    let mut count = 0;
    if first
        && second
        && catch_unwind(AssertUnwindSafe(|| {
            seen += 1;
            probe(seen + limit);
        }))
        .is_err()
        && seen <= limit
    {
        count += 100;
    }
    count + seen
}

/// Its `Ok` asked for, where it didn't throw.
fn quiet(asked: bool, n: u32) -> &'static str {
    if asked && catch_unwind(|| probe(n)).is_ok() {
        return "quiet";
    }
    "loud"
}

/// An `else`, which runs where what's first is false and where it didn't
/// throw: still the `Result`'s test.
fn otherwise(asked: bool, n: u32) -> u32 {
    let mut count = 0;
    if asked && catch_unwind(|| probe(n)).is_err() {
        count += 1;
    } else {
        count += 10;
    }
    count * 2
}

fn main() {
    for n in [1, 3] {
        for can in [true, false] {
            println!("check {can} {n} {}", check(can, n));
            println!("quiet {can} {n} {}", quiet(can, n));
            println!("otherwise {can} {n} {}", otherwise(can, n));
        }
    }
    for first in [true, false] {
        for second in [true, false] {
            for limit in [0, 1, 2] {
                println!("counted {first} {second} {limit} {}", counted(first, second, limit));
            }
        }
    }
}
