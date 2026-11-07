// A clone of many parts whose clone of a later one panics drops what it
// made already: a derived `Clone`'s fields and a tuple's, the latest
// first, as Rust drops an aggregate's operands; a `Vec`'s, a
// `VecDeque`'s, an array's and a map's items in order, as std's clone
// drops what its guard holds, a map's key before the entries made (#897).
// A clone that can't panic, of `Copy` or `String` fields, reads as before.
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug)]
struct D(u32);

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        if self.0 == 0 {
            panic!("clone");
        }
        D(self.0)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Debug)]
struct Pair {
    a: D,
    b: D,
    c: D,
}

#[derive(Clone, Debug)]
enum Shape {
    Dot,
    Line { from: D, to: D },
}

#[derive(Clone, Debug, PartialEq)]
struct Plain {
    n: u32,
    name: String,
}

fn run(name: &str, f: impl FnOnce()) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    run("fields", || {
        let p = Pair { a: D(1), b: D(2), c: D(0) };
        let q = p.clone();
        println!("{:?}", q.a);
    });
    run("variant", || {
        let s = Shape::Line { from: D(1), to: D(0) };
        let t = s.clone();
        println!("{:?}", matches!(t, Shape::Dot));
    });
    run("vec of derived", || {
        let v = vec![
            Pair { a: D(1), b: D(2), c: D(3) },
            Pair { a: D(4), b: D(0), c: D(5) },
        ];
        let w = v.clone();
        println!("{}", w.len());
    });
    run("tuple", || {
        let t = (D(1), D(2), D(0));
        let u = t.clone();
        println!("{}", u.0.0);
    });
    run("vec", || {
        let v = vec![D(1), D(2), D(0), D(3)];
        let w = v.clone();
        println!("{}", w.len());
    });
    run("to_vec", || {
        let v = [D(1), D(0)];
        let w = v.to_vec();
        println!("{}", w.len());
    });
    run("array", || {
        let a = [D(1), D(2), D(0)];
        let b = a.clone();
        println!("{}", b.len());
    });
    run("deque", || {
        let q: VecDeque<D> = [D(1), D(0)].into();
        let r = q.clone();
        println!("{}", r.len());
    });
    run("btree map", || {
        let m: BTreeMap<u32, D> = [(1, D(1)), (2, D(2)), (3, D(0))].into();
        let n = m.clone();
        println!("{}", n.len());
    });
    run("map's pair", || {
        let m: HashMap<u32, (D, D)> = [(1, (D(1), D(0)))].into();
        let n = m.clone();
        println!("{}", n.len());
    });
    run("plain", || {
        let p = Plain { n: 1, name: "one".to_string() };
        let q = p.clone();
        println!("{}", p == q);
    });
}
