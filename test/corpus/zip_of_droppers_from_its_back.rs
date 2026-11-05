// A zip of two `Vec`s' items with a destructor, stepped from its back by a
// `rev()`'s `rfold`, `next_back` and `rfind`: made knowing its back,
// `$zipEnds` given each side's drop, as one of plain items is (#740), not
// std's compiled `Zip` (#812). The longer side is trimmed from its back
// first, each it trims dropped, then the backs paired; what's left is
// dropped as the zip is. A `rev()`'s `rfold` is the zip's `fold`, which
// std's `SpecFold` takes the shorter's count of pairs by, never stepping
// its first side once more as `next()` does: what's left of it is dropped
// as the zip is, after the fold's last pair, where a drop that panics shows
// it; and where its first side is a map, whose closure would run once
// more, it runs as often as std's (ADR 0997).
use std::panic::{AssertUnwindSafe, catch_unwind};

const PANICS: u32 = 99;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
        if self.0 == PANICS {
            panic!("drop");
        }
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        print!("drop acc {} ", self.0);
    }
}

fn ds(from: u32, to: u32) -> Vec<D> {
    (from..to).map(D).collect()
}

fn main() {
    let plain = vec![1u32, 2, 3]
        .into_iter()
        .zip(vec![4u32, 5])
        .rev()
        .rfold(0, |acc, (x, y)| acc * 10 + x + y);
    println!("| plain {plain}");
    let sum = ds(1, 4)
        .into_iter()
        .zip(ds(4, 6))
        .rev()
        .rfold(0, |acc, (x, y)| acc * 10 + x.0 + y.0);
    println!("| rfold {sum}");
    let last = ds(1, 3).into_iter().zip(ds(3, 6)).rev().map(|(x, y)| x.0 * y.0).next();
    println!("| first of rev {last:?}");
    let found = ds(1, 5)
        .into_iter()
        .zip(ds(5, 9))
        .rfind(|(x, _)| x.0 % 2 == 0)
        .map(|(x, y)| (x.0, y.0));
    println!("| rfind {found:?}");
    let mut calls = 0;
    let folded = [1u32, 2, 3]
        .iter()
        .map(|&x| {
            calls += 1;
            x
        })
        .zip(vec![D(7), D(8)])
        .rev()
        .rfold(0, |acc, (x, d)| acc * 10 + x + d.0);
    println!("| mapped {folded} {calls}");
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut sides = ds(1, 5);
        sides[2].0 = PANICS;
        sides
            .into_iter()
            .zip(ds(5, 7))
            .rev()
            .rfold(Acc(0), |acc, (x, y)| Acc(acc.0 * 10 + x.0 + y.0))
            .0
    }));
    println!("| panicking drop {}", r.is_err());
    println!("| end");
}
