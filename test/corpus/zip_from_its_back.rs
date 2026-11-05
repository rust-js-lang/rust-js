// A zip stepped from its back and counted, as std's `Zip` is double-ended
// and exact-sized where both its sides are: `rev()`, `next_back()`,
// `rfind()`, `len()` and a `rev()`'s `nth_back()` of a slice's `copied()`
// or `cloned()` items zipped with a range or another slice's; of items with
// a destructor, the longer side trimmed from its back first, each it trims
// dropped, then the backs paired; held in a variable and stepped from
// either end; and where the second side's step panics, the first's item
// dropped (#740).
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

fn ds(from: u32, to: u32) -> Vec<D> {
    (from..to).map(D).collect()
}

struct Boom {
    left: u32,
}

impl Iterator for Boom {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left as usize, Some(self.left as usize))
    }
}

impl DoubleEndedIterator for Boom {
    fn next_back(&mut self) -> Option<u32> {
        assert!(self.left != 3, "back");
        self.next()
    }
}

impl ExactSizeIterator for Boom {}

fn main() {
    let v = vec![1u32, 2, 3];
    let r: Vec<(u32, i32)> = v.iter().copied().zip(0..2).rev().collect();
    println!("| copied rev {r:?}");
    let r: Vec<(u32, i32)> = v.iter().cloned().zip(0..2).rev().collect();
    println!("| cloned rev {r:?}");
    let found = v.iter().copied().zip(10..13).rfind(|&(x, _)| x < 3);
    println!("| rfind {found:?}");
    let w = [4u32, 5, 6];
    let second = v.iter().zip(w.iter()).rev().nth_back(1);
    println!("| rev nth_back {second:?}");
    let a = ds(1, 4);
    let b = ds(4, 6);
    let n = a.iter().cloned().zip(b.iter().cloned()).len();
    println!("| len {n}");
    let last = a
        .iter()
        .cloned()
        .zip(b.iter().cloned())
        .next_back()
        .map(|(x, y)| (x.0, y.0));
    println!("| next_back {last:?}");
    let last = b
        .iter()
        .cloned()
        .zip(a.iter().cloned())
        .next_back()
        .map(|(x, y)| (x.0, y.0));
    println!("| next_back of the shorter first {last:?}");
    let shorter = v.iter().zip(w.iter().skip(1)).len();
    println!("| len of a skip {shorter}");
    let pairs: Vec<(u32, u32)> = ds(1, 4)
        .into_iter()
        .zip(ds(4, 5))
        .rev()
        .map(|(x, y)| (x.0, y.0))
        .collect();
    println!("| longer first {pairs:?}");
    let pairs: Vec<(u32, u32)> = ds(1, 2)
        .into_iter()
        .zip(ds(4, 7))
        .rev()
        .map(|(x, y)| (x.0, y.0))
        .collect();
    println!("| longer second {pairs:?}");
    let mut it = ds(1, 5).into_iter().zip(ds(5, 8));
    let back = it.next_back().map(|(x, y)| (x.0, y.0));
    let front = it.next().map(|(x, y)| (x.0, y.0));
    println!("| held {back:?} {front:?} {}", it.len());
    drop(it);
    println!("| dropped");
    let r = catch_unwind(AssertUnwindSafe(|| {
        ds(1, 3)
            .into_iter()
            .zip(Boom { left: 3 })
            .next_back()
            .map(|(x, y)| (x.0, y))
    }));
    println!("| panics {}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        a.iter()
            .cloned()
            .zip(Boom { left: 3 })
            .next_back()
            .map(|(x, y)| (x.0, y))
    }));
    println!("| cloned panics {}", r.is_err());
    drop(a);
    drop(b);
    println!("| end");
}
