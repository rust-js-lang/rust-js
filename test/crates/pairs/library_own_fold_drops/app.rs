//! The library's iterator that writes its own `fold`, folded by this
//! crate, directly, by its generic code and through a std adapter; and
//! this crate's own such iterator folded by the library's generic code:
//! each `fold` is given the drops of what it folds into and by, so what a
//! panic in it unwinds past drops as natively (#2271).
use dep::{D, Ds, ds};

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop Acc{}", self.0);
    }
}

/// Counts to 3, its own `fold` panicking at 2.
struct Mine(u32);

impl Iterator for Mine {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("mine fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            if x == 2 {
                panic!("mine");
            }
            acc = f(acc, x);
        }
        acc
    }
}

fn generic<I: Iterator<Item = D>>(it: I) -> u32 {
    it.fold(Acc(100), |acc, d| Acc(acc.0 + d.0)).0
}

fn caught(name: &str, f: impl FnOnce() -> u32 + std::panic::UnwindSafe) {
    println!("{name} {:?}", std::panic::catch_unwind(f).ok());
}

pub fn main() {
    dep::consumed(9);
    dep::consumed(3);
    dep::adapted(9, 1);
    dep::adapted(3, 1);
    dep::adapted(9, 77);
    caught("fold", || {
        Ds { n: 0, end: 4, boom: 3 }.fold(Acc(10), |acc, d| Acc(acc.0 + d.0)).0
    });
    caught("generic", || generic(ds(3)));
    caught("generic whole", || generic(ds(9)));
    caught("mapped", || ds(3).map(|d| d.0).fold(Acc(20), |acc, x| Acc(acc.0 + x)).0);
    caught("library's generic", || dep::total(Mine(0)));
    caught("library's generic mapped", || dep::total(ds(3).map(|d| d.0)));
}
