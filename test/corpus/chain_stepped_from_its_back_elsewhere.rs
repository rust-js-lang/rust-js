// A chain the crate steps from its back in one function is made knowing its
// back wherever its type is made (ADR 0545), and is still std's `Chain`
// from its front in another: its `fold`, `for_each`, `count`, `last`,
// `nth` and `try_fold` are each side's, the crate's own `fold` and `count`
// where it writes them, `a`'s then `b`'s (#739). From its back, its
// `nth_back` drops what it passes of `b` by `b`'s `advance_back_by` (#752),
// and its `rfold` closes `a` as `b`'s panics. Folded by value with a
// closure that drops, each side is closed as its fold is done with it
// (#757); and a map of a chain of the crate's iterators is stepped from
// its back where MIR holds the chain in a variable.
use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own(u32);

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(D(self.0))
    }
    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold");
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
    fn count(self) -> usize {
        println!("own count");
        self.0 as usize
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        self.next()
    }
}

// The crate's iterator that drops, stepped from either end.
struct Backs {
    at: u32,
    end: u32,
}

impl Drop for Backs {
    fn drop(&mut self) {
        println!("drop src");
    }
}

impl Iterator for Backs {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(D(self.at - 1))
    }
}

impl DoubleEndedIterator for Backs {
    fn next_back(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(D(self.end))
    }
}

// Each chain of these types is stepped from its back here.
fn back() -> u32 {
    Own(2).chain(vec![D(7)]).rfold(0, |a, d| a * 10 + d.0)
}

// A chain of chains of the crate's iterators, none with an
// `advance_back_by` of its own: what `nth_back` skips of each side is
// dropped by each `next_back`, the inner chain's by the drop it's given.
fn nested_back() -> Option<u32> {
    Backs { at: 1, end: 3 }
        .chain(Backs { at: 5, end: 8 })
        .chain(Backs { at: 9, end: 10 })
        .nth_back(3)
        .map(|d| d.0)
}

// A chain of the crate's iterators, held before its `rev()`: made knowing
// its back where it's made, each side's too.
fn held_back() -> u32 {
    let chained = Backs { at: 1, end: 3 }.chain(Backs { at: 5, end: 7 });
    chained.rev().fold(0, |a, d| a * 10 + d.0)
}

fn backs_back() -> u32 {
    Backs { at: 1, end: 4 }.chain(vec![D(6)]).rfold(0, |a, x| a + x.0)
}

fn skipped_back(v: Vec<D>, w: Vec<D>) -> Option<u32> {
    v.into_iter().chain(w).nth_back(1).map(|d| d.0)
}

fn main() {
    println!("rfold {}", back());
    println!("fold {}", Own(2).chain(vec![D(7)]).fold(0, |a, d| a * 10 + d.0));
    println!("count {}", Own(2).chain(vec![D(7)]).count());
    let mut sum = 0;
    Own(2).chain(vec![D(7)]).for_each(|d| sum += d.0);
    println!("for_each {sum}");
    println!("last {:?}", Own(2).chain(vec![D(7), D(8)]).last().map(|d| d.0));
    println!("nth {:?}", Own(3).chain(vec![D(7), D(8)]).nth(3).map(|d| d.0));
    println!("any {}", Own(3).chain(vec![D(7), D(8)]).any(|d| d.0 == 7));
    let set: BTreeSet<D> = Own(2).chain(vec![D(7)]).collect();
    println!("set {}", set.len());
    drop(set);
    println!("{:?}", skipped_back(vec![D(1), D(2)], vec![D(6), D(7)]));
    println!("{:?}", skipped_back(vec![D(1), D(2)], vec![D(5), D(6), D(7)]));
    println!("{:?}", skipped_back(vec![D(1), D(2), D(3)], vec![D(6)]));
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1), D(2)].into_iter().chain(vec![D(6), D(7)]).rfold(0, |a, d| {
            if d.0 == 6 {
                panic!("rfold");
            }
            a + d.0
        })
    }));
    println!("rfold ok {}", r.is_ok());
    // Folded by value with a closure that drops, each side closed as its
    // fold is done with it, the crate's own first (#757).
    println!("rfold {}", backs_back());
    let g = D(91);
    let n = Backs { at: 1, end: 4 }.chain(vec![D(6)]).fold(0, move |a, x| {
        let _g = &g;
        a + x.0
    });
    println!("fold {n}");
    let g = D(92);
    let n = Backs { at: 1, end: 3 }.chain(vec![D(6)]).rfold(0, move |a, x| {
        let _g = &g;
        a + x.0
    });
    println!("rfold {n}");
    println!("{:?}", skipped_back(vec![D(1), D(2), D(3)], vec![]));
    println!("{:?}", nested_back());
    println!("{}", held_back());
    // A chain of two of the crate's iterators that drop, under a map that
    // holds what drops, folded from its back.
    let h = D(81);
    let n = Backs { at: 1, end: 3 }
        .chain(Backs { at: 5, end: 7 })
        .map(move |x| {
            let _h = &h;
            x.0
        })
        .rfold(0, |a, x| a * 10 + x);
    println!("map rfold {n}");
    // Its side's fold panicking: that side's rest dropped as its fold
    // unwinds, then the closure, then the other side.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D(93);
        let mut n = 0;
        Backs { at: 1, end: 4 }.chain(vec![D(6)]).fold(0, move |a, x| {
            let _g = &g;
            n += 1;
            if n == 2 {
                panic!("fold");
            }
            a + x.0
        })
    }));
    println!("fold ok {}", r.is_ok());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D(95);
        let mut n = 0;
        vec![D(1), D(2), D(3)].into_iter().chain(vec![D(6)]).fold(0, move |a, x| {
            let _g = &g;
            n += 1;
            if n == 2 {
                panic!("fold");
            }
            a + x.0
        })
    }));
    println!("vec fold ok {}", r.is_ok());
    // A `try_fold` by a closure that drops, whose `b` side's first step
    // panics: `b` is given `f`, so its frame drops `f`, then the
    // accumulator (#768).
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D(96);
        let mut k = 0;
        let skipping = vec![D(2), D(3)].into_iter().skip_while(move |d| {
            k += 1;
            if k == 1 {
                panic!("skip");
            }
            d.0 < 3
        });
        vec![D(1)].into_iter().chain(skipping).try_fold(D(50), move |a, x| {
            let _g = &g;
            Some(D(a.0 + x.0))
        })
    }));
    println!("try_fold ok {}", r.is_ok());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let g = D(94);
        let mut n = 0;
        Backs { at: 1, end: 3 }.chain(vec![D(5), D(6), D(7)]).rfold(0, move |a, x| {
            let _g = &g;
            n += 1;
            if n == 2 {
                panic!("rfold");
            }
            a + x.0
        })
    }));
    println!("rfold ok {}", r.is_ok());
}
