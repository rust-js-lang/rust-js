// A chain whose sides are std's adapters over items with a destructor,
// stepped from its back: MIR holds side `a` in a variable before the chain
// is made, where `b`'s drop flag is cleared, so no form of it is left to
// read. Each side is made knowing its back where it's made, a chain's
// sides as a map's source is (#718): held before its `rev()` or made in
// place, through a `for`, `next`, a sum, `rfold` and `nth_back`, and as a
// predicate panics part way.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(from: u32) -> Vec<D> {
    (from..from + 3).map(D).collect()
}

fn main() {
    let c = ds(1).into_iter().filter(|d| d.0 != 2).chain(ds(4));
    for d in c.rev() {
        println!("for {}", d.0);
    }

    let c = ds(1).into_iter().map(|d| D(d.0 * 10)).chain(ds(4));
    let mut r = c.rev();
    println!("next {:?}", r.next().map(|d| d.0));
    println!("next {:?}", r.next().map(|d| d.0));
    drop(r);

    let n = ds(1)
        .into_iter()
        .filter(|d| d.0 != 2)
        .chain(ds(4))
        .rev()
        .map(|d| d.0)
        .sum::<u32>();
    println!("sum {n}");

    let c = ds(1).into_iter().filter(|d| d.0 % 2 == 1).chain(ds(4));
    let n = c.rfold(0, |a, d| a * 10 + d.0);
    println!("rfold {n}");

    let mut c = ds(1)
        .into_iter()
        .inspect(|d| println!("saw {}", d.0))
        .chain(ds(4).into_iter().map(|d| D(d.0 + 1)));
    println!("nth_back {:?}", c.nth_back(3).map(|d| d.0));
    println!("next {:?}", c.next().map(|d| d.0));
    drop(c);

    let c = ds(1)
        .into_iter()
        .filter_map(|d| (d.0 != 3).then(|| D(d.0 + 100)))
        .chain(ds(4));
    let all: Vec<u32> = c.rev().map(|d| d.0).collect();
    println!("collect {all:?}");

    let caught = catch_unwind(AssertUnwindSafe(|| {
        let c = ds(1).into_iter().filter(|d| {
            assert!(d.0 != 2, "filter");
            true
        });
        let c = c.chain(ds(4));
        for d in c.rev() {
            println!("for {}", d.0);
        }
    }));
    println!("panicked {}", caught.is_err());
}
