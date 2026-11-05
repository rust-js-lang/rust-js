// A `rev()` of a `rev()` of a `Vec`'s items with a destructor is std's
// `Rev<Rev<IntoIter<T>>>`. Its `last` is std's default, a fold that keeps
// each item as it goes, not `IntoIter::last`, which takes the back first.
// Where a drop panics, the item to keep in its place is dropped as it
// unwinds, and nothing is leaked. So it stays `$rev($rev(..))` of them,
// not the `Vec`'s own `IntoIter`; so does its `skip()`'s and its
// `chain()`'s `last` (#722).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn run(name: &str, f: impl FnOnce() -> Option<u32>) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("rev rev", || ds().into_iter().rev().rev().last().map(|d| d.0 + 0));
    run("rev rev skip", || ds().into_iter().rev().rev().skip(1).last().map(|d| d.0 + 0));
    run("rev rev chain", || {
        ds().into_iter()
            .rev()
            .rev()
            .chain(vec![D(6)])
            .last()
            .map(|d| d.0 + 0)
    });
    run("rev rev count", || Some(ds().into_iter().rev().rev().count() as u32));
    run("rev rev nth", || ds().into_iter().rev().rev().nth(1).map(|d| d.0 + 0));
    run("rev rev collect", || Some(ds().into_iter().rev().rev().collect::<Vec<_>>().len() as u32));
    run("rev", || ds().into_iter().rev().last().map(|d| d.0 + 0));
}
