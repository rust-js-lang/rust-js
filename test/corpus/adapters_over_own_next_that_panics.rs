// The crate's iterator, with a `Drop` and its own `fold`, whose `next`
// panics, behind std's adapters, each folded, collected or counted under
// `catch_unwind`: what may panic behind an adapter is the crate's own
// `next`, which no JS shows, so a fold's accumulator, a collect's items
// and what's left are dropped as std unwinds (rd763, rd765); a `map`
// that knows its length folds by its source's `fold` (rd762); an
// `enumerate()` gives its source the accumulator's drop (rd767); a
// generic fold of a `fuse()` drops it once (rd764); and a `skip(n)` whose
// first step panics drops `init`, then itself (rd769).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own {
    items: std::vec::IntoIter<D>,
    at: u32,
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        let next = self.items.next();
        if next.as_ref().is_some_and(|d| d.0 == self.at) {
            panic!("boom");
        }
        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.items.size_hint()
    }

    fn fold<B, F: FnMut(B, D) -> B>(self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        for d in self {
            acc = f(acc, d);
        }
        acc
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        let next = self.items.next_back();
        if next.as_ref().is_some_and(|d| d.0 == self.at) {
            panic!("boom back");
        }
        next
    }
}

impl ExactSizeIterator for Own {}

fn own(r: std::ops::Range<u32>, at: u32) -> Own {
    Own {
        items: r.map(D).collect::<Vec<_>>().into_iter(),
        at,
    }
}

fn push(mut acc: Vec<D>, d: D) -> Vec<D> {
    acc.push(d);
    acc
}

fn fold_any<I: Iterator<Item = D>>(it: I) -> usize {
    it.fold(Vec::new(), push).len()
}

fn fold_exact<I: ExactSizeIterator<Item = D>>(it: I) -> usize {
    it.fold(Vec::new(), push).len()
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("-- {name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("exact map fold", || fold_exact(own(0..3, 99).map(|d| D(d.0 + 1000))));
    run("exact map fold panics", || {
        fold_exact(own(0..3, 1).map(|d| D(d.0 + 1000)))
    });
    run("fuse fold", || own(10..14, 12).fuse().fold(Vec::new(), push).len());
    run("generic fuse fold", || fold_any(own(20..24, 99).fuse()));
    run("generic fuse fold panics", || fold_any(own(30..34, 32).fuse()));
    run("fuse collect", || own(40..44, 42).fuse().collect::<Vec<_>>().len());
    run("skip fold", || own(50..55, 53).skip(1).fold(Vec::new(), push).len());
    run("skip fold in skip", || {
        own(50..55, 50).skip(2).fold(vec![D(59)], push).len()
    });
    run("skip count in skip", || own(50..55, 50).skip(2).count());
    run("step_by fold", || {
        own(60..66, 64).step_by(2).fold(Vec::new(), push).len()
    });
    run("take fold", || own(70..75, 72).take(4).fold(Vec::new(), push).len());
    run("chain fold", || {
        own(80..82, 99).chain(own(83..86, 84)).fold(Vec::new(), push).len()
    });
    run("zip fold", || {
        own(90..94, 92)
            .zip(own(100..104, 99))
            .fold(Vec::new(), |acc, (a, b)| push(push(acc, a), b))
            .len()
    });
    run("peekable fold", || {
        own(110..114, 112).peekable().fold(Vec::new(), push).len()
    });
    run("rev fold", || own(120..124, 121).rev().fold(Vec::new(), push).len());
    run("enumerate fold", || {
        own(130..134, 132)
            .enumerate()
            .fold(Vec::new(), |acc, (_, d)| push(acc, d))
            .len()
    });
    run("try_fold option", || {
        own(140..144, 142)
            .fuse()
            .try_fold(Vec::new(), |acc, d| Some(push(acc, d)))
            .map_or(0, |v| v.len())
    });
    run("try_fold result", || {
        let r: Result<Vec<D>, ()> = own(150..154, 152)
            .fuse()
            .try_fold(Vec::new(), |acc, d| Ok(push(acc, d)));
        r.map_or(0, |v| v.len())
    });
    run("skip collect", || own(160..164, 162).skip(1).collect::<Vec<_>>().len());
    run("chain collect", || {
        own(170..172, 99).chain(own(173..176, 174)).collect::<Vec<_>>().len()
    });
    run("peekable collect", || {
        own(180..184, 182).peekable().collect::<Vec<_>>().len()
    });
    run("take collect", || own(190..195, 192).take(4).collect::<Vec<_>>().len());
    run("rev collect", || own(200..204, 201).rev().collect::<Vec<_>>().len());
    run("enumerate rfold", || {
        own(210..214, 211)
            .enumerate()
            .rfold(Vec::new(), |acc, (_, d)| push(acc, d))
            .len()
    });
    run("map rfold", || {
        own(220..224, 221).map(|d| D(d.0 + 1000)).rfold(Vec::new(), push).len()
    });
}
