// `max()` and `min()` of the crate's iterator of items whose order is
// written by hand, std's `reduce(Ord::max)` since 1.100, `$reduceBy`, with
// a `Drop` that panics as the fold ends: the fold gives back the item it
// kept, then drops `self`, whose panic leaks it, as #1655's `max_by` does
// (ADR 0109, amended). Where a step panics, the kept item still drops,
// before what's left.
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, Debug)]
struct K(u32);

impl PartialOrd for K {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for K {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl Drop for K {
    fn drop(&mut self) {
        println!("drop K {}", self.0);
    }
}

struct Ks {
    at: u32,
    end: u32,
    panics_at: Option<u32>,
    panics_as_dropped: bool,
}

fn ks(end: u32, panics_at: Option<u32>, panics_as_dropped: bool) -> Ks {
    Ks {
        at: 0,
        end,
        panics_at,
        panics_as_dropped,
    }
}

impl Iterator for Ks {
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.at >= self.end {
            return None;
        }
        self.at += 1;
        assert!(Some(self.at) != self.panics_at, "a step");
        Some(K(self.at))
    }
}

impl Drop for Ks {
    fn drop(&mut self) {
        println!("drop Ks");
        if self.panics_as_dropped {
            panic!("as dropped");
        }
    }
}

fn top<I: Iterator<Item = K>>(it: I) -> Option<K> {
    it.max()
}

fn caught<T: std::fmt::Debug>(f: impl FnOnce() -> T) {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{value:?}"),
        Err(_) => println!("panicked"),
    }
}

fn main() {
    caught(|| ks(3, None, true).max().map(|k| k.0));
    caught(|| ks(3, None, true).min().map(|k| k.0));
    caught(|| ks(3, None, true).max_by(|a, b| a.cmp(b)).map(|k| k.0));
    caught(|| ks(3, None, true).min_by(|a, b| a.cmp(b)).map(|k| k.0));
    caught(|| ks(3, None, true).max_by_key(|k| k.0 % 2).map(|k| k.0));
    caught(|| ks(3, None, true).min_by_key(|k| k.0 % 2).map(|k| k.0));
    caught(|| top(ks(3, None, true)).map(|k| k.0));
    caught(|| ks(1, None, true).max().map(|k| k.0));
    caught(|| ks(0, None, true).max().map(|k| k.0));
    caught(|| ks(3, Some(3), false).max().map(|k| k.0));
    caught(|| ks(3, Some(3), false).max_by_key(|k| k.0).map(|k| k.0));
    caught(|| ks(3, None, false).max().map(|k| k.0));
}
