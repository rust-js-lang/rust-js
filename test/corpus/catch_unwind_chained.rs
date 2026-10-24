// A `catch_unwind` that heads an `if let` chain is a `try`, the rest of
// the chain the `catch`'s `if` (ADR 0706, amended): its later conditions read what
// the closure set; one with an `else`, one that reads the payload, and one
// whose `Ok` is read after it, each as Rust runs them.
use std::panic::{AssertUnwindSafe, catch_unwind};

fn step(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    n * 10
}

/// React's `DetermineComponentFrameRoot`: what the closure set, read by
/// the chain once it has thrown.
fn root(n: u32) -> (Option<String>, Option<u32>) {
    let mut control = None;
    if let Err(sample) = catch_unwind(AssertUnwindSafe(|| {
        if n > 3 {
            control = Some(n);
        }
        step(n);
    }))
        // Its message and what was set.
        && let Some(sample) = sample.downcast_ref::<String>()
        && let Some(control) = control
    {
        return (Some(sample.clone()), Some(control));
    }
    (None, None)
}

/// An `else`, which runs where it returned and where the rest is false.
fn otherwise(n: u32, wanted: bool) -> &'static str {
    if let Err(_) = catch_unwind(|| step(n))
        && wanted
    {
        "caught"
    } else {
        "not"
    }
}

/// A chain that goes on after it, where the `if` doesn't return.
fn counted(n: u32, limit: u32) -> u32 {
    let mut count = 0;
    let mut seen = 0;
    if let Err(_) = catch_unwind(AssertUnwindSafe(|| {
        seen += 1;
        step(n)
    })) && seen <= limit
    {
        count += 100;
    }
    count + seen
}

/// An `else` where its arm goes on: each runs once, then what follows.
fn both(n: u32, wanted: bool) -> u32 {
    let mut count = 0;
    if let Err(_) = catch_unwind(|| step(n))
        && wanted
    {
        count += 1;
    } else {
        count += 10;
    }
    count * 2
}

/// A chain an `Ok` heads, whose `else` runs where it threw.
fn ok_chain(n: u32, wanted: bool) -> u32 {
    if let Ok(v) = catch_unwind(|| step(n))
        && wanted
    {
        return v;
    }
    7
}

/// Its `Ok` read after it.
fn kept(n: u32, wanted: bool) -> u32 {
    let result = catch_unwind(|| step(n));
    if let Err(_) = &result
        && wanted
    {
        return 1;
    }
    result.unwrap_or(2)
}

fn main() {
    for n in [1, 3, 4] {
        println!("root {n} {:?}", root(n));
        for wanted in [true, false] {
            println!("otherwise {n} {wanted} {}", otherwise(n, wanted));
            println!("kept {n} {wanted} {}", kept(n, wanted));
            println!("both {n} {wanted} {}", both(n, wanted));
            println!("ok_chain {n} {wanted} {}", ok_chain(n, wanted));
        }
        for limit in [0, 1] {
            println!("counted {n} {limit} {}", counted(n, limit));
        }
    }
}
