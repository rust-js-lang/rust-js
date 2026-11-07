// `o.insert(v)` is `*o = Some(v)`: a drop of the old value that panics
// still writes `v`, as Rust's drop in place of an assignment does, so `v`
// is what's dropped with `o`, and the old value only once (#904).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 1 {
            panic!("drop");
        }
    }
}

struct Holder {
    slot: Option<D>,
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("a local", || {
        let mut o = Some(D(1));
        let r = o.insert(D(9));
        println!("{}", r.0);
    });
    run("a field, then read", || {
        let mut h = Holder { slot: Some(D(1)) };
        let r = catch_unwind(AssertUnwindSafe(|| {
            h.slot.insert(D(8));
        }));
        println!("{} {}", r.is_ok(), h.slot.as_ref().map_or(0, |d| d.0));
    });
    run("none before", || {
        let mut o: Option<D> = None;
        let r = o.insert(D(7));
        println!("{}", r.0);
    });
    run("quiet", || {
        let mut o = Some(D(3));
        let r = o.insert(D(4));
        println!("{}", r.0);
    });
}
