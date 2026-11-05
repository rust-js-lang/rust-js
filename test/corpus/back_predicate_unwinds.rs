// A predicate that panics as a consumer steps from the back: the item it
// was asked of is dropped as it unwinds, then what's left, as std's
// `rfind` holds it in its `check` closure's frame (`try_rfold`), and as
// `rposition`'s, `find_map`'s, `find`'s, `position`'s, `any`'s and
// `all`'s closures own theirs, of a `Vec`'s items, through `flat_map`,
// `by_ref`, `chain` and `filter`, of the crate's iterator and of a type
// parameter (#738).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3)]
}

// The crate's iterator, stepped from either end.
struct Own(u32, u32);

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            D(self.0)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Own {}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            D(self.1 + 1)
        })
    }
}

// The crate's iterator that says when its length is asked: an
// `enumerate()`'s `rfind` asks it once, its `next_back` after each step.
struct Hints(u32);

impl Iterator for Hints {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl ExactSizeIterator for Hints {}

impl DoubleEndedIterator for Hints {
    fn next_back(&mut self) -> Option<D> {
        println!("next_back");
        (self.0 > 0).then(|| {
            self.0 -= 1;
            D(self.0 + 1)
        })
    }
}

// Whether `run` panicked, its predicate panicking at its second call.
fn second<T>(what: &str, run: impl FnOnce(&mut dyn FnMut(&D) -> bool) -> T) {
    let mut n = 0;
    let mut p = |d: &D| {
        n += 1;
        if n == 2 {
            panic!("p");
        }
        d.0 == 9
    };
    let r = catch_unwind(AssertUnwindSafe(|| {
        run(&mut p);
    }));
    println!("{what} ok {}", r.is_ok());
}

fn generic<I: DoubleEndedIterator<Item = D>>(it: I, p: &mut dyn FnMut(&D) -> bool) -> Option<D> {
    let mut it = it;
    it.rfind(|d| p(d))
}

fn main() {
    second("rfind", |p| ds().into_iter().rfind(|d| p(d)).is_some());
    second("flat_map rfind", |p| {
        vec![1, 2].into_iter().flat_map(|k| vec![D(k * 10), D(k * 10 + 1)]).rfind(|d| p(d)).is_some()
    });
    second("by_ref rfind", |p| {
        let mut it = ds().into_iter();
        let found = it.by_ref().rfind(|d| p(d)).is_some();
        println!("left {}", it.len());
        found
    });
    second("chain rfind", |p| ds().into_iter().chain(vec![D(4)]).rfind(|d| p(d)).is_some());
    second("own rfind", |p| Own(0, 3).rfind(|d| p(d)).is_some());
    second("generic rfind", |p| generic(ds().into_iter(), p).is_some());
    second("generic own rfind", |p| generic(Own(0, 3), p).is_some());
    second("rposition", |p| ds().into_iter().rposition(|d| p(&d)));
    second("rev find_map", |p| ds().into_iter().rev().find_map(|d| p(&d).then_some(d.0)));
    second("rev find", |p| ds().into_iter().rev().find(|d| p(d)).is_some());
    second("rev position", |p| ds().into_iter().rev().position(|d| p(&d)));
    second("rev any", |p| ds().into_iter().rev().any(|d| p(&d)));
    second("rev all", |p| ds().into_iter().rev().all(|d| !p(&d)));
    second("own rev find", |p| Own(0, 3).rev().find(|d| p(d)).is_some());
    second("filter next_back", |p| ds().into_iter().filter(|d| !p(d)).next_back().is_some());
    // Std's structs held, whose `rfind` is their own `try_rfold`.
    second("held rfind", |p| {
        let mut held = Own(0, 5).take(4).skip(1);
        held.rfind(|d| p(d)).is_some()
    });
    second("enumerate rfind", |p| Hints(3).enumerate().rfind(|(_, d)| p(d)).is_some());
    let found = Hints(4).enumerate().rfind(|(_, d)| d.0 % 2 == 1);
    println!("{:?}", found.map(|(i, d)| (i, d.0)));
    let mut counted = Hints(3).enumerate();
    println!("{:?}", counted.next_back().map(|(i, d)| (i, d.0)));
    let last = Hints(3).enumerate().next_back();
    println!("{:?}", last.map(|(i, d)| (i, d.0)));
    let first = Hints(3).enumerate().rev().next();
    println!("{:?}", first.map(|(i, d)| (i, d.0)));
    // A filter stepped from either end, each it doesn't keep dropped.
    let mut kept = ds().into_iter().filter(|d| d.0 % 2 == 0);
    println!("{:?}", kept.next_back().map(|d| d.0));
    println!("{:?}", kept.next().map(|d| d.0));
    println!("{:?}", kept.next().map(|d| d.0));
}
