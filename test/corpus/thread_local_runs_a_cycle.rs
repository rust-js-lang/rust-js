// A thread-local is made as its module loads, whatever its initializer runs
// (ADR 1006, amended), but not one whose initializer runs or reads code of
// a module in an import cycle with its own, which may not have loaded yet:
// that one is made where it's first used, as Rust makes it (ADR 0037,
// amended), whatever runs that code: a drop of its type, a fn pointer to
// its function, a print of what it formats, std's `Option::map` given its
// function, a call of it, or a read of its `static`.
use std::cell::Cell;

const BASE: u32 = 3;

mod cycle {
    use std::fmt;

    pub static START: u32 = 10;

    pub struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            println!("guard dropped");
        }
    }

    pub struct Shown;

    impl fmt::Display for Shown {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "shown {}", super::BASE)
        }
    }

    pub fn loud(n: u32) -> u32 {
        println!("loud {n}");
        n + super::BASE
    }
}

thread_local! {
    static DROPPED: u32 = {
        let _guard = cycle::Guard;
        1
    };
    static POINTED: u32 = {
        let loud: fn(u32) -> u32 = cycle::loud;
        loud(2)
    };
    static SHOWN: u32 = {
        println!("{}", cycle::Shown);
        3
    };
    static MAPPED: Option<u32> = Some(4).map(cycle::loud);
    static MAPPED_TWICE: Option<u32> = Some(2).map(|n| {
        println!("mapped {n}");
        cycle::loud(n * 2)
    });
    static CALLED: u32 = cycle::loud(5);
    static STARTED: u32 = cycle::START + 1;
    static COUNTED: Cell<u32> = Cell::new(cycle::loud(6));
}

fn main() {
    println!("main");
    println!("{}", DROPPED.with(|d| *d));
    println!("{}", POINTED.with(|p| *p));
    println!("{}", SHOWN.with(|s| *s));
    println!("{:?}", MAPPED.with(|m| *m));
    println!("{:?}", MAPPED_TWICE.with(|m| *m));
    println!("{}", CALLED.with(|c| *c));
    println!("{}", STARTED.with(|s| *s));
    COUNTED.with(|c| c.set(c.get() + 1));
    println!("{}", COUNTED.with(Cell::get));
}
