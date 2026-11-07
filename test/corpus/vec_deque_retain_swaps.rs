// A `VecDeque`'s `retain_mut`, and its `retain`, are std's: each item kept
// swapped forward into its place, and once every one is asked, what's past
// them truncated, each dropped in the order the swaps left them, after the
// last call; where the closure panics, every item stays in the deque as the
// swaps left it (#909). A `Vec`'s drops each as it goes (ADR 0499).
use std::cell::Cell;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

/// Prints as it's dropped.
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name} {boom}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

/// Asked of each item's number: panics where it's `BOOM`'s, once.
fn keeps(n: u32) -> bool {
    println!("keep? {n}");
    if BOOM.get() == n {
        BOOM.set(0);
        panic!("keep {n}");
    }
    n % 2 == 0
}

fn main() {
    for boom in [0, 2, 5] {
        run("retain_mut", boom, || {
            let mut q: VecDeque<D> = [3, 1, 2, 5, 4, 6].map(D).into();
            q.retain_mut(|x| {
                x.0 += 10;
                keeps(x.0 - 10)
            });
            println!("{q:?}");
        });
        run("retain", boom, || {
            let mut q: VecDeque<D> = [3, 1, 2, 5, 4, 6].map(D).into();
            q.retain(|x| keeps(x.0));
            println!("{q:?}");
        });
        run("Vec retain", boom, || {
            let mut v = vec![D(3), D(1), D(2)];
            v.retain(|x| keeps(x.0));
            println!("{v:?}");
        });
    }
    // Of numbers, through a handle each.
    let mut q: VecDeque<u32> = [1, 2, 3, 4].into();
    q.retain_mut(|x| {
        *x *= 3;
        *x % 2 == 0
    });
    println!("{q:?}");
}
