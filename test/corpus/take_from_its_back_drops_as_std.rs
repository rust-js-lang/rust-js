// A `take(n)` of a `Vec`'s items with a destructor, made knowing its back
// (#718), drops what it passes as std's `Take` does (#826): its `rfold`
// that a stage's panic stops as it trims drops the accumulator, then what's
// left; a `take(n).rev()` collected into a `Vec` is folded, as std's
// `extend_trusted` folds a `TrustedLen` one, so what `rfold` trims drops
// from the back, of a type parameter's too; and its `count` is its
// source's `advance_by`, which drops a reversed `Vec`'s as one slice,
// front to back, where a chain's `rfold` of one elsewhere makes its type
// knowing its back.
use std::panic::catch_unwind;

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
    (1..=5).map(D).collect()
}

fn collected<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> Vec<u32> {
    let v: Vec<D> = src.take(3).rev().collect();
    v.iter().map(|d| d.0).collect()
}

fn main() {
    let stopped = catch_unwind(|| {
        let mut calls = 0;
        let acc = ds()
            .into_iter()
            .rev()
            .map(|d| {
                calls += 1;
                if calls == 2 {
                    panic!("map");
                }
                D(d.0 + 100)
            })
            .take(3)
            .rfold(Acc(0), |mut a, d| {
                a.0 += d.0;
                a
            });
        println!("rfold {}", acc.0);
    });
    println!("stopped {}", stopped.is_err());

    let v: Vec<D> = ds().into_iter().take(3).rev().collect();
    println!("collected {:?}", v.iter().map(|d| d.0).collect::<Vec<_>>());
    drop(v);
    println!("generic {:?}", collected(ds().into_iter()));

    let chained = ds()
        .into_iter()
        .rev()
        .take(3)
        .chain(vec![D(6), D(7)])
        .rfold(0, |sum, d| sum + d.0);
    println!("chained {chained}");
    let n = ds().into_iter().rev().take(3).enumerate().count();
    println!("count {n}");
}
