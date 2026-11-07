// A `BinaryHeap` whose comparison panics in a sift keeps every item, once:
// std's sift holds the item it moves in a `Hole`, written back as it
// unwinds, so the heap drops each once; `pop` drops the top it took, and
// `into_sorted_vec` and `from` drop what they hold, as their frames unwind
// (#901).
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
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

fn heap(items: &[u32]) -> BinaryHeap<D> {
    let boom = BOOM.replace(0);
    let mut h = BinaryHeap::new();
    for &n in items {
        h.push(D(n));
    }
    BOOM.set(boom);
    h
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name} {boom}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn main() {
    for boom in 0..7 {
        run("push", boom, || {
            let mut h = heap(&[3, 1, 4, 1, 5]);
            h.push(D(9));
            println!("{}", h.len());
        });
        run("pop", boom, || {
            let mut h = heap(&[3, 1, 4, 1, 5, 9, 2]);
            let top = h.pop();
            println!("{:?} {}", top, h.len());
        });
        run("pop of many", boom, || {
            let mut h = heap(&[8, 3, 12, 1, 15, 6, 10, 2, 14, 5, 9, 13, 4, 11, 7]);
            let top = h.pop();
            println!("{:?} {}", top, h.len());
        });
        run("append", boom, || {
            let mut h = heap(&[3, 1, 4]);
            let mut o = heap(&[2, 7]);
            h.append(&mut o);
            println!("{} {}", h.len(), o.len());
        });
        run("extend", boom, || {
            let mut h = heap(&[3, 1, 4]);
            h.extend(vec![D(2), D(5)]);
            println!("{}", h.len());
        });
        run("into_sorted_vec", boom, || {
            let h = heap(&[3, 1, 4, 2]);
            let v = h.into_sorted_vec();
            println!("{:?}", v);
        });
        run("from", boom, || {
            let h = BinaryHeap::from(vec![D(3), D(1), D(4), D(2), D(5)]);
            println!("{}", h.len());
        });
        run("pop of a struct", boom, || {
            let mut h = BinaryHeap::from(vec![p(3), p(1), p(4)]);
            let top = h.pop();
            println!("{:?} {}", top.map(|p| p.a.0), h.len());
        });
        run("into_sorted_vec of a struct", boom, || {
            let h = BinaryHeap::from(vec![p(3), p(1), p(4)]);
            println!("{}", h.into_sorted_vec().len());
        });
    }
}

/// A struct whose derived order is its `D`'s, then its number's.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct P {
    a: D,
    b: u32,
}

fn p(n: u32) -> P {
    P { a: D(n), b: n }
}
