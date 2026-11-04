// The crate's own `fold`, whose `next` panics part-way, reached through
// std's adapters and through generic code: the accumulator it holds as it
// unwinds dropped first, by the drop its fold is given, then what's left
// (#615). Generic code given a `copied()` that knows its length, or one
// stepped from its back, folds a JS iterator made of what it's given; of
// std's struct, a `fuse()`'s, a `skip()`'s or a `take()`'s, its fold is
// the struct's, which passes the drop on to the crate's own.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone)]
struct Nums<'a> {
    items: std::slice::Iter<'a, u32>,
    at: u32,
}

impl<'a> Iterator for Nums<'a> {
    type Item = &'a u32;

    fn next(&mut self) -> Option<&'a u32> {
        let next = self.items.next();
        if next.is_some_and(|n| *n == self.at) {
            panic!("boom");
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.items.size_hint()
    }

    fn fold<B, F: FnMut(B, &'a u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(n) = self.next() {
            acc = f(acc, n);
        }
        acc
    }
}

impl ExactSizeIterator for Nums<'_> {}

impl DoubleEndedIterator for Nums<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.items.next_back()
    }
}

fn push(mut acc: Vec<D>, n: u32) -> Vec<D> {
    acc.push(D(n));
    acc
}

fn copied_fold<'a, I: ExactSizeIterator<Item = &'a u32> + Clone>(it: I) -> usize {
    let copied = it.copied();
    println!("len {}", copied.len());
    copied.fold(Vec::new(), push).len()
}

fn back_fold<'a, I: DoubleEndedIterator<Item = &'a u32> + Clone>(mut it: I) -> usize {
    println!("back {:?}", it.next_back());
    it.copied().fold(Vec::new(), push).len()
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    let nums = [1, 2, 3, 4, 5];
    let made = || Nums {
        items: nums.iter(),
        at: 3,
    };
    run("map", || made().map(|n| *n).fold(Vec::new(), push).len());
    run("inspect", || {
        made().inspect(|_| {}).copied().fold(Vec::new(), push).len()
    });
    run("copied fuse", || copied_fold(made().fuse()));
    run("copied skip", || copied_fold(made().skip(1)));
    run("copied take", || copied_fold(made().take(4)));
    run("copied own", || copied_fold(made()));
    run("back fuse", || back_fold(made().fuse()));
    run("back own", || back_fold(made()));
}
