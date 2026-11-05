// A `scan()` of the crate's iterator with a destructor drops it once, where
// Rust drops the `Scan`: consumed by value, `count`, `sum`, `fold`, `last`,
// `for_each` and `collect`, whose fold takes it whole, so nothing after
// drops it again (#741, #748); held and dropped, of a `Vec`'s items too;
// and as a panic in its closure unwinds, the item it held first, then
// what's left of its source, then what a collect collected (#746).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct It {
    left: u32,
    _held: D,
}

impl Iterator for It {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}

fn it(left: u32) -> It {
    It { left, _held: D("src") }
}

struct Item(u32);

impl Drop for Item {
    fn drop(&mut self) {
        println!("drop item {}", self.0);
    }
}

fn items() -> Vec<Item> {
    (1..=5).map(Item).collect()
}

fn consumed() {
    let n = it(3)
        .scan(0u32, |s, x| {
            *s += 1;
            (*s < 3).then_some(x)
        })
        .count();
    println!("count {n}");
    let n: u32 = it(3)
        .scan(0u32, |s, x| {
            *s += 1;
            Some(x + *s)
        })
        .sum();
    println!("sum {n}");
    let n = it(3)
        .scan(0u32, |s, x| {
            *s += x;
            Some(*s)
        })
        .fold(0, |a, x| a + x);
    println!("fold {n}");
    let last = it(3).scan(0u32, |s, x| {
        *s += 1;
        Some(x * *s)
    });
    println!("last {:?}", last.last());
    it(3)
        .scan(0u32, |s, x| {
            *s += 1;
            Some(x)
        })
        .for_each(|x| println!("each {x}"));
    let v: Vec<u32> = it(4)
        .scan(0u32, |s, x| {
            *s += 1;
            (*s < 3).then_some(x)
        })
        .collect();
    println!("collect {v:?}");
}

fn held() {
    let mut held = it(3).scan(0u32, |s, x| {
        *s += 1;
        Some(x)
    });
    println!("next {:?}", held.next());
    drop(held);
    println!("dropped");
    // Of a `Vec`'s items, dropped unstepped, and after a step.
    drop(items().into_iter().scan(0, |_, x| Some(x)));
    println!("unstepped");
    let mut stepped = items().into_iter().scan(0, |_, x| Some(x));
    println!("first {:?}", stepped.next().map(|x| x.0));
    drop(stepped);
    println!("stepped");
}

fn panics() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        it(3)
            .scan(0u32, |s, x| {
                *s += 1;
                assert!(*s != 2, "scan");
                Some(x)
            })
            .count()
    }));
    println!("count panicked {}", r.is_err());
    let mut calls = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v: Vec<Item> = items()
            .into_iter()
            .rev()
            .scan(0u32, |s, x| {
                calls += 1;
                assert!(calls != 2, "scan");
                *s += 1;
                (*s < 4).then_some(x)
            })
            .collect();
        v.len()
    }));
    println!("rev collect panicked {}", r.is_err());
    calls = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v: Vec<Item> = items()
            .into_iter()
            .scan((), |_, x| {
                calls += 1;
                assert!(calls != 3, "scan");
                Some(x)
            })
            .collect();
        v.len()
    }));
    println!("collect panicked {}", r.is_err());
}

fn main() {
    consumed();
    held();
    panics();
}
