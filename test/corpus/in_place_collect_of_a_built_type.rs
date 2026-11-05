// A collect into a `Vec` in generic code, whose items are of a type built
// of its type parameters, `Held<<H::Item as IntoIterator>::IntoIter>`, as
// itertools' `multi_cartesian_product` collects (multi_product.rs:66):
// whether std collects it in place, in its source's buffer, is its
// caller's, of the source's items' layout and the destination's. Where a
// panic stops it, in place it drops what it collected first, then what's
// left; else what's left first (#672, #737).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I> {
    iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

fn held<H>(iters: H) -> Vec<Held<<H::Item as IntoIterator>::IntoIter>>
where
    H: Iterator,
    H::Item: IntoIterator,
{
    iters.map(|i| Held { iter: i.into_iter() }).collect()
}

/// An `Option`, whose iterator is as big and as aligned, so std collects
/// what holds it in place.
fn wide(w: Option<D>) -> Option<D> {
    if w.as_ref().is_some_and(|d| d.0 == 3) {
        panic!("wide");
    }
    w
}

/// A `Vec`, whose iterator is bigger, so std never collects what holds it
/// in place.
fn narrow(v: Vec<D>) -> Vec<D> {
    if v[0].0 == 3 {
        panic!("narrow");
    }
    v
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("wide", || {
        held(vec![Some(D(1)), None].into_iter())
            .iter()
            .map(|h| h.iter.len())
            .sum()
    });
    run("wide, a panic", || {
        let wides: Vec<Option<D>> = (1..=4).map(|n| Some(D(n))).collect();
        held(wides.into_iter().map(wide)).len()
    });
    run("narrow", || {
        held(vec![vec![D(1)], vec![D(2), D(3)]].into_iter())
            .iter()
            .map(|h| h.iter.len())
            .sum()
    });
    run("narrow, a panic", || {
        let narrows: Vec<Vec<D>> = (1..=4).map(|n| vec![D(n)]).collect();
        held(narrows.into_iter().map(narrow)).len()
    });
    run("not in place", || held((1..=2).map(|n| vec![D(n)])).len());
}
