//! A consumer of the library's `held` and `tagged`, giving each the layout
//! of its items, the library's manifest's template of their type made of
//! what it gives: in place, a panic drops what it collected first; not in
//! place, what's left (#737).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn wide(w: Option<D>) -> Option<D> {
    if w.as_ref().is_some_and(|d| d.0 == 3) {
        panic!("wide");
    }
    w
}

fn narrow(v: Vec<D>) -> Vec<D> {
    if v[0].0 == 3 {
        panic!("narrow");
    }
    v
}

fn tag(d: (D, u64, u64)) -> (D, u64, u64) {
    if d.0.0 == 3 {
        panic!("tag");
    }
    d
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

pub fn main() {
    run("wide, a panic", || {
        let wides: Vec<Option<D>> = (1..=4).map(|n| Some(D(n))).collect();
        dep::held(wides.into_iter().map(wide)).len()
    });
    run("narrow, a panic", || {
        let narrows: Vec<Vec<D>> = (1..=4).map(|n| vec![D(n)]).collect();
        dep::held(narrows.into_iter().map(narrow)).len()
    });
    run("not in place", || dep::held((1..=2).map(|n| vec![D(n)])).len());
    run("tagged, a panic", || {
        let triples: Vec<(D, u64, u64)> = (1..=4).map(|n| (D(n), n, n)).collect();
        dep::tagged(triples.into_iter().map(|t| tag(t).0)).len()
    });
    run("tagged", || {
        let tagged = dep::tagged([D(5), D(6)].into_iter());
        tagged
            .iter()
            .map(|(d, a, z)| d.0 as usize + a[2] as usize + **z as usize)
            .sum()
    });
}
