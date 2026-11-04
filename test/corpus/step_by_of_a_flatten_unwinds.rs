// A `step_by()` of a `flatten()` or a `flat_map()` steps it by `nth`, as
// std's `StepBy` does, so what the inner iterator it's in hasn't given
// stays in it: where the consumer panics, the source's rest is dropped
// first, then the inner one's, as the `Flatten`'s fields drop (#668).
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=4).map(D).collect()
}

fn pairs(d: D) -> Vec<D> {
    vec![D(d.0 + 10), d]
}

fn nested() -> Vec<Vec<D>> {
    ds().into_iter().map(pairs).collect()
}

fn run(name: &str, f: impl FnOnce() -> u32) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("fold", || {
        let mut seen = 0;
        let acc = nested().into_iter().flatten().step_by(2).fold(Acc(0), |mut a, d| {
            seen += 1;
            if seen == 2 {
                panic!("fold");
            }
            a.0 += d.0;
            a
        });
        acc.0
    });
    run("flat_map fold", || {
        let mut seen = 0;
        let acc = ds().into_iter().flat_map(pairs).step_by(3).fold(Acc(0), |mut a, d| {
            seen += 1;
            if seen == 2 {
                panic!("flat_map");
            }
            a.0 += d.0;
            a
        });
        acc.0
    });
    run("rev collect", || {
        let mut seen = 0;
        let s: String = (nested().into_iter().rev().flatten().step_by(2))
            .map(|d| {
                seen += 1;
                if seen == 2 {
                    panic!("collect");
                }
                char::from(b'a' + d.0 as u8 % 26)
            })
            .collect();
        s.len() as u32
    });
    run("set", || {
        let set: HashSet<u32> = (nested().into_iter().flatten().step_by(2))
            .map(|d| {
                if d.0 == 12 {
                    panic!("set");
                }
                d.0
            })
            .collect();
        set.len() as u32
    });
    run("no panic", || {
        let acc = (nested().into_iter().flatten().step_by(3)).fold(Acc(0), |mut a, d| {
            a.0 += d.0;
            a
        });
        acc.0
    });
}
