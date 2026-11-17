// A guard inlined where it's made restores what it saved, read as the
// variable it was made of, `previous`, by that variable and not by its
// word: a block beside it declares another `previous`, and a closure's
// parameter is one more, each its own variable, which the restore never
// reads (ADR 1150).
use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let Self { previous } = *self;
        println!("restored {previous} (was {})", DEPTH.get());
        DEPTH.set(previous);
    }
}

fn enter(n: u32) -> u32 {
    let previous = DEPTH.replace(n);
    let _restore = Restore { previous };
    let total = {
        let previous = previous * 10 + n;
        previous + DEPTH.get()
    };
    let scaled = [1, 2].map(|previous| previous * total);
    println!("inside {total} {scaled:?} at {}", DEPTH.get());
    total
}

fn main() {
    println!("{}", enter(3));
    DEPTH.set(7);
    println!("{}", enter(5));
    println!("left at {}", DEPTH.get());
}
