// `last()` of items whose destructor may panic: the item it kept is
// dropped as the next replaces it, and where that drop panics it isn't
// dropped again as the panic unwinds; the one it was to keep in its place
// is the closure's, which std's `last` leaks there (#648). In place, of a
// `fuse()`, and in generic code.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("drop");
        }
    }
}

fn last<I: Iterator<Item = D>>(it: I) -> Option<u32> {
    it.last().map(|d| d.0 + 0)
}

fn run(name: &str, f: impl FnOnce() -> Option<u32>) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("in place", || vec![D(1), D(2), D(3)].into_iter().last().map(|d| d.0 + 0));
    run("fused", || vec![D(1), D(2), D(3)].into_iter().fuse().last().map(|d| d.0 + 0));
    run("generic", || last(vec![D(1), D(2), D(3)].into_iter()));
    run("generic enumerated", || last(vec![D(1), D(2), D(3)].into_iter().enumerate().map(|(_, d)| d)));
    run("last kept", || last(vec![D(3), D(1)].into_iter()));
}
