// A flatten's `fold` and `rfold` are std's `FlattenCompat`'s: its source's
// own `fold` or `rfold` of each item's inner, each inner's own, the crate's
// where it writes one, as a `rev()` of it asks them too (#621, #649). What
// they fold is dropped as Rust drops it, though a closure panics.
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's iterator of `D(a + 1)` to `D(b)`, which writes its own
/// `fold` and `rfold`.
struct Folds(u32, u32);

impl Drop for Folds {
    fn drop(&mut self) {
        println!("drop folds {}", self.0);
    }
}

impl Iterator for Folds {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }
    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

impl DoubleEndedIterator for Folds {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
    fn rfold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own rfold");
        let mut acc = init;
        while let Some(x) = self.next_back() {
            acc = f(acc, x);
        }
        acc
    }
}

/// Each item's pair, `[D(10 * n), D(n)]`.
fn pair(d: D) -> Vec<D> {
    vec![D(d.0 * 10), d]
}

/// Generic code's flatten of what it's given, folded from its back.
fn rfolded<I: DoubleEndedIterator<Item = D>>(it: I) -> u32 {
    it.map(pair).flatten().rfold(0, |a, d| a * 2 + d.0)
}

fn main() {
    let sum = |a: u32, d: D| a * 2 + d.0;
    println!("rfold {}", Folds(0, 3).map(pair).flatten().rfold(0, sum));
    println!("flat_map rfold {}", Folds(0, 3).flat_map(pair).rfold(0, sum));
    println!("rev fold {}", Folds(0, 3).map(pair).flatten().rev().fold(0, sum));
    println!("rev rfold {}", Folds(0, 3).map(pair).flatten().rev().rfold(0, sum));
    println!("fold {}", Folds(0, 3).map(pair).flatten().fold(0, sum));
    println!("generic {}", rfolded(Folds(0, 3)));

    // Each inner's own.
    let inners = vec![Folds(0, 2), Folds(5, 7)];
    println!("inners rfold {}", inners.into_iter().flatten().rfold(0, sum));
    let inners = vec![Folds(0, 2), Folds(5, 7)];
    println!("inners fold {}", inners.into_iter().flatten().fold(0, sum));

    // A closure that panics part-way: what's left of the inner it's in,
    // then the crate's iterator, dropped as they unwind.
    let ended = catch_unwind(|| {
        let n = Folds(0, 3).map(pair).flatten().rfold(0, |a, d| {
            if d.0 == 20 {
                panic!("rfold");
            }
            a + d.0
        });
        println!("never {n}");
    });
    println!("panicked {}", ended.is_err());
    let ended = catch_unwind(|| {
        let n = Folds(0, 3).map(pair).flatten().fold(0, |a, d| {
            if d.0 == 2 {
                panic!("fold");
            }
            a + d.0
        });
        println!("never {n}");
    });
    println!("panicked {}", ended.is_err());
}
