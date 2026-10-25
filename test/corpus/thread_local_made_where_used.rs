// A thread-local whose initializer runs the crate's code, or may panic, is
// made where it's first used, as Rust makes each (ADR 0037, amended): what
// it prints after what runs before its first use, and what it drops; one
// whose initializer panics made again at its next use; and one a module's load would make
// before another module it calls has loaded (#207), made only once both
// have. One that only makes a value is its module's own, made as it loads,
// which nothing can tell.

use std::cell::Cell;

fn noisy(n: u32) -> u32 {
    println!("made {n}");
    n
}

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        println!("guard dropped");
    }
}

thread_local! {
    static LOUD: Cell<u32> = Cell::new(noisy(7));
    static PLAIN: Cell<u32> = Cell::new(3);
    // A fn pointer's call runs what it may be, the crate's `noisy` here
    // (ADR 0907): made where it's first used.
    static POINTED: u32 = {
        let made: fn(u32) -> u32 = noisy;
        made(9)
    };
    // Another module's `const`, read by its name, as written.
    static FROM_LIMITS: u32 = noisy(limits::LIMIT);
    // What it drops runs the crate's code too: made where it's first used.
    static GUARDED: u32 = {
        let _guard = Guard;
        5
    };
    static TRIES: Cell<u32> = const { Cell::new(0) };
    static FLAKY: u32 = {
        let n = TRIES.with(|t| {
            t.set(t.get() + 1);
            t.get()
        });
        println!("try {n}");
        if n == 1 {
            panic!("first try");
        }
        n * 10
    };
}

// `limits` and `start` import each other: `start`'s thread-local calls a
// function of `limits` that reads its `const`.
mod limits {
    pub const LIMIT: u32 = 3;

    pub fn limit_plus_one() -> u32 {
        let _ = super::start::read;
        let _ = super::around;
        LIMIT + 1
    }
}

thread_local! {
    // Of this module, which `limits` calls back into, so it may load after
    // it: read by its name too, as it runs once both have (#292).
    static AROUND: u32 = limits::limit_plus_one() + limits::LIMIT;
}

fn around() -> u32 {
    AROUND.with(|a| *a)
}

mod start {
    use super::limits::{LIMIT, limit_plus_one};

    thread_local! {
        // Its initializer reads `LIMIT` by its name, as written: it runs once
        // both modules have loaded.
        pub static START: u32 = limit_plus_one() + LIMIT;
    }

    pub fn read() -> u32 {
        START.with(|s| *s)
    }
}

fn main() {
    println!("main");
    println!("{}", LOUD.with(|l| l.get()));
    LOUD.with(|l| l.set(8));
    println!("{} {}", LOUD.with(|l| l.get()), PLAIN.with(|p| p.get()));
    println!("{}", GUARDED.with(|g| *g));
    println!("{}", POINTED.with(|p| *p));
    println!("{}", FROM_LIMITS.with(|f| *f));
    println!("{}", around());

    let first = std::panic::catch_unwind(|| FLAKY.with(|f| *f));
    println!("{}", first.is_err());
    println!("{} {}", FLAKY.with(|f| *f), FLAKY.with(|f| *f));

    println!("{}", start::read());
}
