// A `fuse()`, a `peekable()`, a `chain()` or a `zip()` of a `Vec`'s
// `into_iter()`, mapped by a closure that panics, collected into a
// `String`, a `HashSet` or a `HashMap`, or its `last()` taken: what the
// source has left is dropped as the panic unwinds, as Rust drops the
// iterator the collect holds (#653).
use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(base: u32) -> Vec<D> {
    vec![D(base + 1), D(base + 2), D(base + 3)]
}

fn main() {
    let r = catch_unwind(|| {
        let s: String = ds(0)
            .into_iter()
            .fuse()
            .map(|d| {
                if d.0 == 2 {
                    panic!("map")
                } else {
                    char::from(b'a' + d.0 as u8)
                }
            })
            .collect();
        s
    });
    println!("fuse string {}", r.is_err());
    let r = catch_unwind(|| {
        let s: HashSet<u32> = ds(10)
            .into_iter()
            .peekable()
            .map(|d| if d.0 == 12 { panic!("map") } else { d.0 })
            .collect();
        s.len()
    });
    println!("peekable set {}", r.is_err());
    let r = catch_unwind(|| {
        let s: HashMap<u32, u32> = ds(20)
            .into_iter()
            .chain(ds(30))
            .map(|d| if d.0 == 22 { panic!("map") } else { (d.0, d.0) })
            .collect();
        s.len()
    });
    println!("chain map {}", r.is_err());
    let r = catch_unwind(|| {
        let s: String = ds(40)
            .into_iter()
            .zip(ds(50))
            .map(|(a, b)| {
                if a.0 == 42 {
                    panic!("map")
                } else {
                    char::from(b'a' + (a.0 + b.0) as u8 % 26)
                }
            })
            .collect();
        s
    });
    println!("zip string {}", r.is_err());
    let r = catch_unwind(|| {
        ds(60)
            .into_iter()
            .fuse()
            .map(|d| if d.0 == 62 { panic!("map") } else { d.0 })
            .count()
    });
    println!("fuse count {}", r.is_err());
    let r = catch_unwind(|| {
        ds(70)
            .into_iter()
            .peekable()
            .map(|d| if d.0 == 72 { panic!("map") } else { d.0 })
            .last()
    });
    println!("peekable last {}", r.is_err());
    let mut log = Vec::new();
    let r = catch_unwind(AssertUnwindSafe(|| {
        let s: String = ds(80)
            .into_iter()
            .fuse()
            .map(|d| char::from(b'a' + d.0 as u8 % 26))
            .collect();
        log.push(s);
    }));
    println!("fuse string no panic {} {:?}", r.is_err(), log);
}
