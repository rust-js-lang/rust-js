// `max`, `min` and `clamp` of a type with its own `Ord` compare as std's
// defaults do, `max(a, b)`'s `b < a`, and drop what they don't give back;
// as a comparison panics, each drops all it was given, the later first,
// as std drops its parameters (#912).
use std::cell::Cell;
use std::cmp::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        self.0 == other.0
    }
}

impl Eq for D {}

impl PartialOrd for D {
    fn partial_cmp(&self, other: &D) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for D {
    fn cmp(&self, other: &D) -> Ordering {
        println!("cmp {} {}", self.0, other.0);
        let n = BOOM.get();
        if n > 0 {
            BOOM.set(n - 1);
            if n == 1 {
                panic!("cmp");
            }
        }
        self.0.cmp(&other.0)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct P {
    a: D,
    b: u32,
}

/// Prints its order as it's compared, and has nothing to drop.
#[derive(Debug, PartialEq, Eq)]
struct Q(u32);

impl PartialOrd for Q {
    fn partial_cmp(&self, other: &Q) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Q {
    fn cmp(&self, other: &Q) -> Ordering {
        println!("cmp q {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    for boom in 0..4 {
        run("max", boom, || println!("{:?}", std::cmp::max(D(1), D(2))));
        run("max of equals", boom, || println!("{:?}", std::cmp::max(D(2), D(2))));
        run("min", boom, || println!("{:?}", std::cmp::min(D(1), D(2))));
        run("Ord::max", boom, || println!("{:?}", D(3).max(D(1))));
        run("Ord::min", boom, || println!("{:?}", D(3).min(D(1))));
        run("clamp", boom, || println!("{:?}", D(4).clamp(D(3), D(5))));
        run("clamp below", boom, || println!("{:?}", D(1).clamp(D(3), D(5))));
        run("clamp above", boom, || println!("{:?}", D(9).clamp(D(3), D(5))));
        run("max of a struct", boom, || {
            println!("{:?}", std::cmp::max(P { a: D(1), b: 0 }, P { a: D(2), b: 0 }))
        });
        run("clamp of a struct", boom, || {
            let p = P { a: D(4), b: 0 }.clamp(P { a: D(3), b: 0 }, P { a: D(5), b: 0 });
            println!("{:?}", p)
        });
    }
    println!("{:?}", std::cmp::max(Q(1), Q(2)));
    println!("{:?}", Q(5).min(Q(4)));
    println!("{:?}", std::cmp::max(Q(3), Q(3)));
}
