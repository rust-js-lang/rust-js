// A `Fuse` is std's own (the fuse-is-std's decision): its `next`,
// `next_back`, `rfold`, `size_hint`, `len`, its clone and its `Debug` are
// std's bodies, compiled from the MIR the toolchain ships into
// `@rust-js/std`, over `{ iter }`. Which of std's two `FuseImpl`s steps it
// is picked by whether what it holds is a `FusedIterator`: one that is, is
// kept as it ends, and shown; another is dropped as it ends. In generic
// code, the caller says which, `IFused`.
use std::iter::{Fuse, FusedIterator};

#[derive(Clone, Debug)]
struct Steady(u32);

impl Iterator for Steady {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.saturating_sub(1);
        (self.0 > 0).then_some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.saturating_sub(1) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Steady {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

impl ExactSizeIterator for Steady {}

impl FusedIterator for Steady {}

#[derive(Clone, Debug)]
struct Flip(u32);

impl Iterator for Flip {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        if self.0 % 2 == 0 { None } else { Some(self.0) }
    }
}

impl DoubleEndedIterator for Flip {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

// Stepped to its end from the front, then shown: a `FusedIterator`'s is
// `Some` still, another's `None`.
fn drained<I: Iterator + std::fmt::Debug>(it: I) -> (usize, String) {
    let mut f = it.fuse();
    let mut n = 0;
    while f.next().is_some() {
        n += 1;
    }
    f.next();
    (n, format!("{f:?}"))
}

// From its back, counted, cloned, and folded from its back.
fn ends<I: DoubleEndedIterator<Item = u32> + ExactSizeIterator + Clone>(it: I) -> (Option<u32>, usize, Vec<u32>, u32) {
    let mut f: Fuse<I> = it.fuse();
    let last = f.next_back();
    let left = f.len();
    let copy = f.clone();
    let rest: Vec<u32> = copy.rev().collect();
    let total = f.rfold(0, |acc, x| acc * 10 + x);
    (last, left, rest, total)
}

// A `Fuse` a struct holds, stepped from its back to its end by the struct's
// impl, which asks `IFused` of its parameter, and one made of what an
// `IntoIterator` makes, held as it is: a `FusedIterator`'s is kept and
// shown.
struct Held<I> {
    f: Fuse<I>,
}

impl<I: DoubleEndedIterator + std::fmt::Debug> Held<I> {
    fn drained_back(&mut self) -> String {
        while self.f.next_back().is_some() {}
        format!("{:?}", self.f)
    }

    // By the `next` it was made with, which knows what it holds.
    fn drained_front(&mut self) -> String {
        while self.f.next().is_some() {}
        format!("{:?}", self.f)
    }
}

fn fused_of<I: IntoIterator>(i: I) -> Held<I::IntoIter> {
    Held { f: i.into_iter().fuse() }
}

fn main() {
    println!("{}", Held { f: Steady(3).fuse() }.drained_back());
    println!("{}", fused_of(Steady(3)).drained_back());
    println!("{}", fused_of(Steady(3)).drained_front());
    println!("{}", fused_of(Flip(0)).drained_back());
    println!("{:?}", drained(Steady(4)));
    println!("{:?}", drained(Flip(0)));
    println!("{:?}", drained(1..3));
    println!("{:?}", ends(Steady(4)));
    println!("{:?}", ends(1..4u32));
    let mut f = Flip(0).fuse();
    println!("{:?} {:?} {:?} {:?}", f.next(), f.next(), f.next(), f);
    let mut s = vec![1, 2].into_iter().fuse();
    println!("{:?} {:?} {:?} {:?}", s.size_hint(), s.next(), s.next(), s);
    println!("{:?} {:?} {}", s.next(), s.size_hint(), s.len());
}
