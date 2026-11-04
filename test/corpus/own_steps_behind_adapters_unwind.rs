// The crate's iterators whose `next` panics, of borrowed numbers with
// their own `fold` and of plain ones, behind std's adapters and behind an
// `impl Iterator` and a `Box<dyn Iterator>`, folded into a `Vec` of `D`s
// under `catch_unwind`: the accumulator a panicking step strands is
// dropped as std unwinds, of a `copied()`, a `fuse()`, a `skip`, a
// `step_by`, a `peekable()`, a `take`, a `zip` and a `chain` (rd768), of
// an `enumerate()`'s fold (rd767), and of a `try_fold` into an `Option`,
// with `?` too; and a map of an enumerate folds by its source's `fold`
// (rd766).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone)]
struct Nums<'a> {
    items: std::slice::Iter<'a, u32>,
    at: u32,
}

impl<'a> Iterator for Nums<'a> {
    type Item = &'a u32;

    fn next(&mut self) -> Option<&'a u32> {
        let next = self.items.next();
        if next.is_some_and(|n| *n == self.at) {
            panic!("boom");
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.items.size_hint()
    }

    fn fold<B, F: FnMut(B, &'a u32) -> B>(self, init: B, mut f: F) -> B {
        println!("nums fold");
        let mut acc = init;
        for n in self {
            acc = f(acc, n);
        }
        acc
    }
}

impl ExactSizeIterator for Nums<'_> {}

impl DoubleEndedIterator for Nums<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.items.next_back()
    }
}

// Its `next` panics, and nothing more.
struct Plain {
    n: u32,
    at: u32,
}

impl Iterator for Plain {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        if self.n == self.at {
            panic!("plain");
        }
        (self.n < 6).then_some(self.n)
    }
}

// Its steps never panic: a fold of it behind std's adapters drops nothing
// as it unwinds, so it's `reduce`.
struct Calm {
    left: bool,
}

impl Iterator for Calm {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.left {
            self.left = false;
            Some(7)
        } else {
            None
        }
    }
}

fn push(mut acc: Vec<D>, n: u32) -> Vec<D> {
    acc.push(D(n));
    acc
}

fn opaque(at: u32) -> impl Iterator<Item = u32> {
    Plain { n: 0, at }
}

fn boxed(at: u32) -> Box<dyn Iterator<Item = u32>> {
    Box::new(Plain { n: 0, at })
}

fn tried(it: impl Iterator<Item = u32>) -> Option<usize> {
    let v = it.fuse().try_fold(Vec::new(), |acc, n| Some(push(acc, n)))?;
    Some(v.len())
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    let nums = [1, 2, 3, 4, 5];
    let made = || Nums {
        items: nums.iter(),
        at: 3,
    };
    run("enumerate map", || {
        made()
            .enumerate()
            .map(|(i, n)| *n * 10 + i as u32)
            .fold(Vec::new(), push)
            .len()
    });
    run("enumerate fold", || {
        made().enumerate().fold(Vec::new(), |acc, (_, n)| push(acc, *n)).len()
    });
    run("map fold", || made().map(|n| n * 2).fold(Vec::new(), push).len());
    run("copied fold", || made().copied().fold(Vec::new(), push).len());
    run("inspect copied fold", || {
        made().inspect(|_| {}).copied().fold(Vec::new(), push).len()
    });
    run("copied collect", || made().copied().map(D).collect::<Vec<_>>().len());
    run("plain fold", || Plain { n: 0, at: 3 }.fold(Vec::new(), push).len());
    run("plain fuse fold", || {
        Plain { n: 0, at: 3 }.fuse().fold(Vec::new(), push).len()
    });
    run("plain skip fold", || {
        Plain { n: 0, at: 3 }.skip(1).fold(Vec::new(), push).len()
    });
    run("plain skip panics in skip", || {
        Plain { n: 0, at: 1 }.skip(2).fold(Vec::new(), push).len()
    });
    run("plain step_by fold", || {
        Plain { n: 0, at: 4 }.step_by(2).fold(Vec::new(), push).len()
    });
    run("plain peekable fold", || {
        Plain { n: 0, at: 3 }.peekable().fold(Vec::new(), push).len()
    });
    run("plain take fold", || {
        Plain { n: 0, at: 3 }.take(4).fold(Vec::new(), push).len()
    });
    run("plain zip fold", || {
        Plain { n: 0, at: 3 }
            .zip(Plain { n: 0, at: 9 })
            .fold(Vec::new(), |acc, (a, b)| push(push(acc, a), b))
            .len()
    });
    run("plain chain fold", || {
        (1..3).chain(Plain { n: 0, at: 2 }).fold(Vec::new(), push).len()
    });
    run("opaque fuse fold", || opaque(3).fuse().fold(Vec::new(), push).len());
    run("dyn fuse fold", || boxed(3).fuse().fold(Vec::new(), push).len());
    run("dyn fold", || boxed(3).fold(Vec::new(), push).len());
    run("try_fold ?", || tried(Plain { n: 0, at: 3 }).unwrap_or(0));
    run("plain fuse collect", || {
        Plain { n: 0, at: 3 }.fuse().map(D).collect::<Vec<_>>().len()
    });
    run("plain skip collect", || {
        Plain { n: 0, at: 3 }.skip(1).map(D).collect::<Vec<_>>().len()
    });
    run("calm fuse fold", || {
        Calm { left: true }.fuse().fold(Vec::new(), push).len()
    });
    run("plain try_fold option", || {
        Plain { n: 0, at: 3 }
            .try_fold(Vec::new(), |acc, n| Some(push(acc, n)))
            .map_or(0, |v| v.len())
    });
}
