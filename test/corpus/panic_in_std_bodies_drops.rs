// A closure that panics inside std's own compiled bodies, a `Fuse`'s and a
// `Peekable`'s over an `Inspect`: the item the inspecting closure was given
// dropped as std's frames unwind, then what's left, as native Rust drops
// them, std's bodies lowered from a `core` built to unwind (#424).
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
    run("fuse for", || {
        for d in ds(30..34).into_iter().inspect(|d| boom(d, 32)).fuse() {
            println!("saw {}", d.0);
        }
    });
    run("peekable next", || {
        let mut p = ds(40..44).into_iter().inspect(|d| boom(d, 41)).peekable();
        let a = p.next();
        println!("{}", a.is_some());
        let b = p.next();
        println!("{}", b.is_some());
    });
}
