// A `collect()` into a `BinaryHeap`, a `BTreeSet` or a `BTreeMap` asks
// what it steps its hint as a `Vec` would, at its first item and each time
// it's full, as std's `from_iter` of each collects a `Vec` first: the
// heap's `BinaryHeap::from(vec)`, a B-tree's sorted `bulk_build` (#797).
// Where a hint panics as it grows, the item just taken is dropped, then
// what's left, then what it collected, as a `Vec`'s is.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// A crate's iterator of `end` items made by `make`, whose `size_hint`
/// prints, says nothing of what's left, and panics at `panics_at`.
struct Own<F> {
    at: u32,
    end: u32,
    panics_at: u32,
    make: F,
}

impl<T, F: FnMut(u32) -> T> Iterator for Own<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.at += 1;
        (self.at <= self.end).then(|| (self.make)(self.at))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint at {}", self.at);
        if self.at == self.panics_at {
            panic!("hint");
        }
        (0, None)
    }
}

fn own<T>(end: u32, make: impl FnMut(u32) -> T) -> Own<impl FnMut(u32) -> T> {
    Own { at: 0, end, panics_at: 0, make }
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("heap", || own(10, |n| n).collect::<BinaryHeap<u32>>().len());
    run("set", || own(10, |n| n % 7).collect::<BTreeSet<u32>>().len());
    run("map", || own(10, |n| (n % 4, n)).collect::<BTreeMap<u32, u32>>().len());
    run("deque", || own(10, |n| n).collect::<VecDeque<u32>>().len());
    run("boxed", || own(10, |n| n).collect::<Box<[u32]>>().len());
    run("list", || own(10, |n| n).collect::<LinkedList<u32>>().len());
    run("mapped heap", || own(10, |n| n).map(|n| n * 2).collect::<BinaryHeap<u32>>().len());
    run("heap, a hint panics as it grows", || {
        let mut it = own(9, D);
        it.panics_at = 5;
        it.collect::<BinaryHeap<D>>().len()
    });
    run("set, a hint panics as it grows", || {
        let mut it = own(9, D);
        it.panics_at = 5;
        it.collect::<BTreeSet<D>>().len()
    });
    run("map, a hint panics as it grows", || {
        let mut it = own(9, |n| (n, D(n)));
        it.panics_at = 5;
        it.collect::<BTreeMap<u32, D>>().len()
    });
    run("generic heap", || heaped(own(10, |n| n)).len());
    run("generic set", || sorted(own(10, |n| n % 7)).len());
    // Of std's own iterators, whose hints the program doesn't see.
    let heap: BinaryHeap<u32> = (1..=5).map(|n| n * 3).collect();
    println!("{:?}", heap.into_sorted_vec());
    let set: BTreeSet<u32> = vec![3, 1, 2, 1].into_iter().collect();
    println!("{set:?}");
    let map: BTreeMap<u32, &str> = [(2, "b"), (1, "a"), (2, "c")].into_iter().collect();
    println!("{map:?}");
}

/// Generic code's collects, of what its caller gives, whose hint its
/// caller's dictionary passes on.
fn heaped<I: Iterator<Item = u32>>(it: I) -> BinaryHeap<u32> {
    it.collect()
}

fn sorted<I: Iterator<Item = u32>>(it: I) -> BTreeSet<u32> {
    it.collect()
}
