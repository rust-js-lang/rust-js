// `by_ref()` of a type parameter given as it is, summed: std's `&mut I`
// steps it by its `next`, `try_fold`'s, never the crate's own `fold`, which
// would take it and drop it before the sum's done; it's dropped where its
// function ends (ADR 0928, #994).
use std::iter::Sum;

struct Every {
    at: u32,
    end: u32,
}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop every at {}", self.at);
    }
}

impl Iterator for Every {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.at < self.end).then(|| {
            self.at += 1;
            self.at
        })
    }

    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
}

struct Total(u32);

impl Sum<u32> for Total {
    fn sum<I: Iterator<Item = u32>>(iter: I) -> Total {
        Total(iter.fold(0, |a, b| a + b))
    }
}

fn summed<I: Iterator<Item = u32>>(src: I, _other: Option<I>) -> u32 {
    let mut held = src;
    let total: Total = held.by_ref().sum();
    println!("total {}", total.0);
    let plain: u32 = held.by_ref().sum();
    println!("then {plain}");
    total.0
}

fn main() {
    println!("{}", summed(Every { at: 0, end: 3 }, None));
    let mut every = Every { at: 0, end: 4 };
    let total: Total = every.by_ref().sum();
    println!("own {} left at {}", total.0, every.at);
}
