// A reversed `Vec`'s `take(n).count()`: std's `Take::count` is its
// source's `advance_by`, a `Rev`'s, its `IntoIter`'s `advance_back_by`,
// which drops what it passes as one slice, front to back, then the rest
// as the iterator is dropped (#670). An item whose drop panics doesn't stop
// the slice's drop, nor the rest's as `count` unwinds, of a destructor
// that may panic as of one that can't, and a `skip` or an `enumerate`
// between them asks the same.
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.get() {
            BOOM.set(0);
            panic!("drop {}", self.0);
        }
    }
}

struct Quiet(u32);

impl Drop for Quiet {
    fn drop(&mut self) {
        println!("quiet {}", self.0);
    }
}

fn five() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn run(name: &str, boom: u32, f: impl FnOnce() -> usize) {
    println!("-- {name}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("count", 0, || five().into_iter().rev().take(3).count());
    run("count past its end", 0, || five().into_iter().rev().take(7).count());
    run("count panics", 3, || five().into_iter().rev().take(3).count());
    run("count panics first", 5, || five().into_iter().rev().take(2).count());
    run("skip count", 0, || five().into_iter().rev().skip(1).take(2).count());
    run("enumerate count", 4, || {
        five().into_iter().rev().take(3).enumerate().count()
    });
    let quiet: Vec<Quiet> = (1..=4).map(Quiet).collect();
    println!("quiet {}", quiet.into_iter().rev().take(2).count());
}
