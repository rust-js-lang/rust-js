// A `filter()`, a `map()` and an `enumerate()` made knowing their back, of
// a source whose back the crate steps or may: each asks its source's
// `size_hint` as std's does, a `Vec`'s collect of a filter `(0, hi)`, a
// `String`'s of a map its source's, and a filter's `count` first, which the
// crate's own iterator prints; and a filter's `try_fold` is its source's,
// so a panic in what it discards, a drop's or its predicate's, leaks the
// accumulator rustc leaks, at every point a panic can be. Each was caught
// only by the iterator tables (#1109): `filter-ends-hint-dropped`,
// `map-ends-hint-dropped`, `filter-ends-count-hint-unasked`,
// `filter-ends-try-fold-by-next`.
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};

static POINT: AtomicU32 = AtomicU32::new(0);
static POINTS: AtomicU32 = AtomicU32::new(0);

/// A point: where it's the run's `k`th, a panic, once.
fn point() {
    let n = POINTS.fetch_add(1, Ordering::Relaxed) + 1;
    if n == POINT.load(Ordering::Relaxed) {
        POINT.store(0, Ordering::Relaxed);
        println!("panic at {n}");
        panic!("point");
    }
}

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        point();
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

/// The crate's own iterator, which says when its hint is asked.
struct Every {
    at: u32,
    end: u32,
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
        println!("own size_hint {n}");
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

fn every() -> Every {
    Every { at: 1, end: 6 }
}

fn odd_ones<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> Vec<D> {
    src.filter(|x| x.0 % 2 == 1).collect()
}

fn odd_count<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) -> usize {
    src.filter(|x| x.0 % 2 == 1).count()
}

fn folded<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: I) {
    let mut seen = 0;
    let r = src
        .filter(|x| {
            point();
            x.0 % 2 == 1
        })
        .try_fold(Acc(0), |mut a, x| {
            point();
            seen += 1;
            a.0 += x.0;
            if seen == 3 { Err(a.0) } else { Ok(a) }
        });
    match &r {
        Ok(a) => println!("ok {}", a.0),
        Err(n) => println!("err {n}"),
    }
}

fn main() {
    let odd = odd_ones(every());
    println!("vec {:?}", odd);
    drop(odd);
    println!("count {}", odd_count(every()));
    let s: String = every().enumerate().map(|(i, d)| char::from(b'a' + (i as u32 + d.0) as u8)).collect();
    println!("string {s:?}");
    let mut k = 1;
    loop {
        POINTS.store(0, Ordering::Relaxed);
        POINT.store(k, Ordering::Relaxed);
        println!("point {k}");
        let ended = catch_unwind(|| folded(vec![D(1), D(2), D(3), D(4), D(5), D(6), D(7)].into_iter()));
        let fired = POINT.load(Ordering::Relaxed) == 0;
        POINT.store(0, Ordering::Relaxed);
        println!("{}", if ended.is_ok() { "ended" } else { "panicked" });
        if !fired || k == 60 {
            break;
        }
        k += 1;
    }
}
