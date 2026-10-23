// A guard whose scope holds an `if`, a loop and a `return` from inside them
// is one `try { .. } finally { drop }` (ADR 0693): it's dropped once on each
// way out, the `return`'s after its value is made, the loop's `break`, the
// scope's end, and a panic's, as Rust drops it.
use std::cell::Cell;
use std::panic;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct Restore {
    previous: u32,
}

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restore {} (was {})", self.previous, DEPTH.get());
        DEPTH.set(self.previous);
    }
}

fn step(n: u32) -> u32 {
    if n == 13 {
        panic!("unlucky {n}");
    }
    n * 2
}

fn find(limit: u32, wanted: u32) -> String {
    {
        let previous = DEPTH.get();
        DEPTH.set(previous + 1);
        let _restore = Restore { previous };
        let first = step(limit);
        if first > 3 {
            let mut i = 0;
            while i < limit {
                if step(i) == wanted {
                    return format!("found {i} at depth {}", DEPTH.get());
                }
                if i > 15 {
                    break;
                }
                i += 1;
            }
        }
    }
    format!("none at depth {}", DEPTH.get())
}

fn main() {
    println!("{}", find(5, 6));
    println!("{}", find(12, 100));
    println!("{}", find(1, 2));
    let caught = panic::catch_unwind(|| find(20, 1000));
    println!("caught {} at depth {}", caught.is_err(), DEPTH.get());
}
