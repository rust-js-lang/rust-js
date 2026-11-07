// A derived `Default`, a tuple's or an array's, whose `default()` of a
// later part panics drops the parts it made already, the latest first, as
// Rust drops an aggregate's operands: `mem::take` and
// `unwrap_or_default` of one (#902). A `Default` that can't panic, of
// numbers and `String`s, reads as before.
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU32, Ordering};

static MADE: AtomicU32 = AtomicU32::new(0);

struct D(u32);

impl Default for D {
    fn default() -> D {
        let n = MADE.fetch_add(1, Ordering::Relaxed) + 1;
        println!("default {n}");
        if n % 3 == 0 {
            panic!("default");
        }
        D(n)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Default)]
struct Three {
    a: D,
    b: D,
    c: D,
}

#[derive(Default)]
struct Outer {
    first: D,
    inner: Three,
}

#[derive(Default, Debug, PartialEq)]
struct Plain {
    n: u32,
    name: String,
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    MADE.store(0, Ordering::Relaxed);
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("struct", || {
        let t = Three::default();
        println!("{}", t.a.0);
    });
    run("nested", || {
        let o = Outer::default();
        println!("{}", o.first.0);
    });
    run("take", || {
        let mut t = Three { a: D(10), b: D(20), c: D(30) };
        let old = std::mem::take(&mut t);
        println!("{} {}", old.a.0, t.a.0);
    });
    run("tuple", || {
        let t: (D, D, D) = Default::default();
        println!("{}", t.0.0);
    });
    run("array", || {
        let a: [D; 3] = Default::default();
        println!("{}", a.len());
    });
    run("unwrap_or_default", || {
        let none: Option<Three> = None;
        let t = none.unwrap_or_default();
        println!("{}", t.b.0);
    });
    run("plain", || {
        let p = Plain::default();
        println!("{}", p == Plain { n: 0, name: String::new() });
    });
}
