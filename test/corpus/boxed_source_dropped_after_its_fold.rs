// Generic code's `Box<I>` folded by value, through an adapter the crate
// also steps from its back, drops `I` after the fold's loop, as std's
// default `fold(self)` drops the box once it has what it folded: where
// that drop panics, the accumulator, `last`'s item and `sum`'s total are
// leaked, never dropped, as rustc leaks what a function gives back as a
// later drop unwinds, where the fold dropped `I` at its end inside its
// last step, as a step's panic, and dropped them (#2232).
use std::iter::Sum;
use std::panic::{AssertUnwindSafe, catch_unwind};

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

struct Total(u32);

impl Drop for Total {
    fn drop(&mut self) {
        println!("drop total {}", self.0);
    }
}

impl Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Total {
        iter.fold(Total(0), |mut t, d| {
            t.0 += d.0;
            t
        })
    }
}

struct Every {
    at: u32,
    end: u32,
    panics: bool,
}

impl Every {
    fn new(panics: bool) -> Self {
        Every { at: 1, end: 4, panics }
    }
}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop src");
        if self.panics {
            panic!("drop src");
        }
    }
}

impl Iterator for Every {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(D(self.at - 1))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Every {}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(D(self.end))
    }
}

fn folded<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: Box<I>) {
    let acc = src
        .inspect(|x| println!("saw {}", x.0))
        .fold(Acc(0), |mut a, x| {
            a.0 += x.0;
            a
        });
    println!("fold {}", acc.0);
}

fn last<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: Box<I>) {
    let l = src.inspect(|x| println!("saw {}", x.0)).last();
    println!("last {:?}", l.as_ref().map(|d| d.0));
}

fn summed<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: Box<I>) {
    let t: Total = src.inspect(|x| println!("saw {}", x.0)).sum();
    println!("sum {}", t.0);
}

fn from_its_back<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: Box<I>) {
    let mut it = src.inspect(|x| println!("saw {}", x.0));
    println!("back {:?}", it.next_back().map(|d| d.0));
    println!("then {:?}", it.next().map(|d| d.0));
}

fn main() {
    for panics in [false, true] {
        let cases: [(&str, fn(Box<Every>)); 4] = [
            ("fold", folded),
            ("last", last),
            ("sum", summed),
            ("from its back", from_its_back),
        ];
        for (name, case) in cases {
            println!("case {name}, panics {panics}");
            let r = catch_unwind(AssertUnwindSafe(|| case(Box::new(Every::new(panics)))));
            println!("panicked {}", r.is_err());
        }
    }
}
