// A copy of a thread-local's module variable read in a `catch_unwind`'s
// `try` is the variable where it's read before anything there runs code,
// as React's `startIsomorphicDefaultIndicatorIfNeeded` calls its
// indicator; read again after a call that may set the variable, or in the
// `catch`, after any of the `try` ran, it's a copy of its own, as Rust
// reads what it copied (ADR 0277, amended).
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static INDICATOR: Cell<Option<fn() -> u32>> = const { Cell::new(None) };
    static PENDING: Cell<Option<u32>> = const { Cell::new(None) };
    static LEVEL: Cell<u32> = const { Cell::new(0) };
}

fn first() -> u32 {
    INDICATOR.set(Some(second));
    1
}

fn second() -> u32 {
    2
}

fn failing() -> u32 {
    INDICATOR.set(Some(second));
    panic!("indicator failed");
}

fn raise() -> u32 {
    LEVEL.set(LEVEL.get() + 1);
    if LEVEL.get() > 1 {
        panic!("too high");
    }
    LEVEL.get()
}

fn start() {
    if let Some(indicator) = INDICATOR.get()
        && PENDING.get().is_none()
    {
        match panic::catch_unwind(AssertUnwindSafe(indicator)) {
            Ok(cleanup) => PENDING.set(Some(cleanup)),
            Err(_) => PENDING.set(Some(0)),
        }
    }
}

fn start_twice() -> u32 {
    let Some(indicator) = INDICATOR.get() else {
        return 0;
    };
    match panic::catch_unwind(AssertUnwindSafe(|| indicator() * 10 + indicator())) {
        Ok(both) => both,
        Err(_) => 0,
    }
}

fn raised() -> u32 {
    let level = LEVEL.get();
    match panic::catch_unwind(raise) {
        Ok(n) => n * 10 + level,
        Err(_) => level + 100,
    }
}

fn raised_caught() -> u32 {
    let level = LEVEL.get();
    match panic::catch_unwind(raise) {
        Ok(n) => n,
        Err(_) => level + 100,
    }
}

fn main() {
    INDICATOR.set(Some(first));
    start();
    println!("{:?} {}", PENDING.get(), INDICATOR.get().map_or(0, |f| f()));
    INDICATOR.set(Some(first));
    println!("{}", start_twice());
    PENDING.set(None);
    INDICATOR.set(Some(failing));
    start();
    println!("{:?} {}", PENDING.get(), INDICATOR.get().map_or(0, |f| f()));
    println!("{} {}", raised(), LEVEL.get());
    println!("{} {}", raised(), LEVEL.get());
    LEVEL.set(1);
    println!("{} {}", raised_caught(), LEVEL.get());
}
