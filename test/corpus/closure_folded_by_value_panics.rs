// A `map`'s closure folded by value whose own call panics: the fold std
// moved it into drops it before what the `take` left, as it unwinds (#710).
// Not of std's compiled structs yet, whose call given a closure by value
// drops it only as it returns.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("d{} ", self.0);
    }
}

// What reads the guard the closure holds, so it's moved in whole.
fn zero(_guard: &D) -> u32 {
    0
}

fn main() {
    let guard = D(9);
    let r = catch_unwind(AssertUnwindSafe(|| {
        let n: u32 = (1..6)
            .map(D)
            .collect::<Vec<_>>()
            .into_iter()
            .take(3)
            .map(move |d| {
                assert!(d.0 != 2, "two");
                d.0 + zero(&guard)
            })
            .sum();
        print!("| {n} | ");
    }));
    println!("{}", if r.is_err() { "panicked" } else { "" });
}
