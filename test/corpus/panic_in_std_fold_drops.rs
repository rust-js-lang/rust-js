// A fold of std's own compiled bodies, a `Fuse`'s and a `Peekable`'s over
// an `Inspect`, whose inspecting closure panics: the accumulator, holding
// what it was given, dropped as std's frames unwind, then what's left, as
// native Rust drops them (#424).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(r: std::ops::Range<u32>) -> Vec<D> {
    r.map(D).collect()
}

fn boom(d: &D, at: u32) {
    if d.0 == at {
        panic!("boom");
    }
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("fuse fold", || {
        let kept = ds(50..54)
            .into_iter()
            .inspect(|d| boom(d, 52))
            .fuse()
            .fold(Vec::new(), |mut kept, d| {
                kept.push(d);
                kept
            });
        println!("{}", kept.len());
    });
    run("peekable fold", || {
        let kept = ds(60..64)
            .into_iter()
            .inspect(|d| boom(d, 62))
            .peekable()
            .fold(Vec::new(), |mut kept, d| {
                kept.push(d);
                kept
            });
        println!("{}", kept.len());
    });
}
