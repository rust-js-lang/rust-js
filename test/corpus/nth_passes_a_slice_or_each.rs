// What an `nth`, an `nth_back` or a `skip` passes over is dropped as its
// source drops it: a `Vec`'s, an array's and a `VecDeque`'s `IntoIter` as
// one slice, which goes on past a panicking item, the rest of it dropped
// before the panic unwinds; a `Drain`'s, a B-tree's and a `BinaryHeap`'s
// each as it's passed, so a panic stops it there (#703, #704, #648).
// Each directly, through a `by_ref()`, a `rev()`, and in generic code.
use std::cell::Cell;
use std::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.get() {
            BOOM.set(0);
            panic!("drop {}", self.0);
        }
    }
}

fn seven() -> Vec<D> {
    (1..=7).map(D).collect()
}

fn nth_of<I: Iterator<Item = D>>(mut it: I, k: usize) -> u32 {
    it.nth(k).map_or(0, |d| d.0)
}

fn nth_back_of<I: DoubleEndedIterator<Item = D>>(mut it: I, k: usize) -> u32 {
    it.nth_back(k).map_or(0, |d| d.0)
}

fn rev_nth_of<I: DoubleEndedIterator<Item = D>>(it: I, k: usize) -> u32 {
    it.rev().nth(k).map_or(0, |d| d.0)
}

/// An iterator a struct holds, which its generic methods step.
struct Holder<I> {
    it: I,
}

impl<I: ExactSizeIterator<Item = D>> Holder<I> {
    fn nth(&mut self, k: usize) -> u32 {
        self.it.nth(k).map_or(0, |d| d.0)
    }

    fn nth_passed_on(&mut self, k: usize) -> u32 {
        nth_and_len(&mut self.it, k)
    }
}

fn nth_and_len<I: ExactSizeIterator<Item = D>>(mut it: I, k: usize) -> u32 {
    let d = it.nth(k).map_or(0, |d| d.0);
    d + it.len() as u32
}

impl<I: DoubleEndedIterator<Item = D> + ExactSizeIterator> Holder<I> {
    fn nth_back(&mut self, k: usize) -> u32 {
        self.it.nth_back(k).map_or(0, |d| d.0)
    }
}

fn run(name: &str, boom: u32, f: impl FnOnce() -> u32) {
    println!("-- {name}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{name} {:?}", r.ok());
}

/// `f` of `it`, held outside what unwinds, so what's left of it after a
/// panic shows how much it passed: then the rest, dropped.
fn held<I: ExactSizeIterator>(name: &str, boom: u32, mut it: I, f: impl FnOnce(&mut I) -> u32) {
    println!("-- {name}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(|| f(&mut it)));
    BOOM.set(0);
    println!("{name} {:?}, {} left", r.ok(), it.len());
}

fn main() {
    held("held vec nth", 2, seven().into_iter(), |it| {
        it.nth(3).map_or(0, |d| d.0)
    });
    held("held vec skip next", 2, seven().into_iter(), |it| {
        it.skip(3).next().map_or(0, |d| d.0)
    });
    held("held vec generic nth", 2, seven().into_iter(), |it| nth_of(it, 3));
    held(
        "held array generic nth",
        2,
        [D(1), D(2), D(3), D(4), D(5)].into_iter(),
        |it| nth_of(it, 3),
    );
    held("held deque nth", 2, VecDeque::from(seven()).into_iter(), |it| {
        it.nth(3).map_or(0, |d| d.0)
    });
    held(
        "held heap generic nth",
        6,
        BinaryHeap::from(seven()).into_iter(),
        |it| nth_of(it, 3),
    );
    held("held vec nth_back", 6, seven().into_iter(), |it| {
        it.nth_back(3).map_or(0, |d| d.0)
    });
    held("held vec by_ref rev nth", 6, seven().into_iter(), |it| {
        it.by_ref().rev().nth(3).map_or(0, |d| d.0)
    });
    let mut h = Holder {
        it: seven().into_iter(),
    };
    held("held holder nth", 2, 0..1, |_| h.nth(3));
    println!("holder {} left", h.it.len());
    let mut h = Holder {
        it: seven().into_iter(),
    };
    held("held holder nth passed on", 2, 0..1, |_| h.nth_passed_on(3));
    println!("holder {} left", h.it.len());
    let mut h = Holder {
        it: seven().into_iter(),
    };
    held("held holder nth_back", 6, 0..1, |_| h.nth_back(3));
    println!("holder {} left", h.it.len());
    let mut v = seven();
    let mut h = Holder { it: v.drain(..) };
    held("held holder drain nth_back", 6, 0..1, |_| h.nth_back(3));
    println!("holder {} left", h.it.len());
    drop(h);
    let mut it = seven().into_iter();
    BOOM.set(2);
    let r = catch_unwind(AssertUnwindSafe(|| it.nth(3).map_or(0, |d| d.0)));
    BOOM.set(0);
    println!("own nth {:?}, {} left", r.ok(), it.len());
    drop(it);
    let mut v = seven();
    held("held drain nth", 2, v.drain(..), |it| it.nth(3).map_or(0, |d| d.0));
    let mut v = seven();
    held("held drain nth_back", 6, v.drain(..), |it| {
        it.nth_back(3).map_or(0, |d| d.0)
    });
    let mut v = seven();
    held("held drain rev nth", 6, v.drain(..), |it| {
        it.rev().nth(3).map_or(0, |d| d.0)
    });
    // As one slice, front to back, past the panic.
    run("vec nth", 2, || seven().into_iter().nth(3).map_or(0, |d| d.0));
    run("vec by_ref skip next", 2, || {
        let mut it = seven().into_iter();
        let d = it.by_ref().skip(3).next().map_or(0, |d| d.0);
        d + it.len() as u32
    });
    run("vec generic nth", 2, || nth_of(seven().into_iter(), 3));
    run("array generic nth", 2, || {
        nth_of([D(1), D(2), D(3), D(4), D(5)].into_iter(), 3)
    });
    run("deque generic nth", 2, || {
        nth_of(VecDeque::from(seven()).into_iter(), 3)
    });
    run("heap generic nth", 6, || {
        nth_of(BinaryHeap::from(seven()).into_iter(), 3)
    });
    run("vec nth_back", 6, || seven().into_iter().nth_back(3).map_or(0, |d| d.0));
    run("vec generic nth_back", 6, || nth_back_of(seven().into_iter(), 3));
    run("vec generic rev nth", 6, || rev_nth_of(seven().into_iter(), 3));
    // Each as it's passed, stopped at the panic.
    run("drain nth", 2, || {
        let mut v = seven();
        let d = v.drain(..).nth(3).map_or(0, |d| d.0);
        d + v.len() as u32
    });
    run("drain nth_back", 6, || {
        let mut v = seven();
        let d = v.drain(..).nth_back(3).map_or(0, |d| d.0);
        d + v.len() as u32
    });
    run("drain rev nth", 6, || {
        let mut v = seven();
        let d = v.drain(..).rev().nth(3).map_or(0, |d| d.0);
        d + v.len() as u32
    });
    run("drain generic nth_back", 6, || {
        let mut v = seven();
        let d = nth_back_of(v.drain(..), 3);
        d + v.len() as u32
    });
    run("drain generic rev nth", 6, || {
        let mut v = seven();
        let d = rev_nth_of(v.drain(..), 3);
        d + v.len() as u32
    });
    let set = || seven().into_iter().collect::<BTreeSet<_>>();
    run("btree nth", 2, || set().into_iter().nth(3).map_or(0, |d| d.0));
    run("btree nth_back", 6, || set().into_iter().nth_back(3).map_or(0, |d| d.0));
    run("btree rev nth", 6, || set().into_iter().rev().nth(3).map_or(0, |d| d.0));
    run("btree generic nth", 2, || nth_of(set().into_iter(), 3));
    run("btree generic nth_back", 6, || nth_back_of(set().into_iter(), 3));
    run("btree generic rev nth", 6, || rev_nth_of(set().into_iter(), 3));
    run("no panic", 0, || {
        nth_of(seven().into_iter(), 3) + nth_back_of(seven().into_iter(), 3)
    });
}
