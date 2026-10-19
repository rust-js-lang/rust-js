// `fuse()` is Rust's `Fuse`: it steps its iterator till it ends, then never
// again, one that starts over too; of one that isn't a `FusedIterator`, it
// drops it as it ends, and its `Debug` shows `None` then; its `size_hint`,
// `Clone` and `Debug` are std's, in generic code too (ADR 0473).
use std::iter::{Fuse, FusedIterator};

struct Loud(&'static str, u32);

impl Iterator for Loud {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.1 = self.1.saturating_sub(1);
        (self.1 > 0).then_some(self.1)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Steady(u32);

impl Iterator for Steady {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.saturating_sub(1);
        (self.0 > 0).then_some(self.0)
    }
}

impl Drop for Steady {
    fn drop(&mut self) {
        println!("drop steady");
    }
}

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

#[derive(Clone, Debug)]
struct Pair<I> {
    i: Fuse<I>,
}

impl<I: Iterator> Iterator for Pair<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.i.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.i.size_hint()
    }
}

fn pair<I: IntoIterator>(i: I) -> Pair<I::IntoIter> {
    Pair {
        i: i.into_iter().fuse(),
    }
}

fn main() {
    let mut f = Flip(0).fuse();
    println!("{f:?}");
    let copy = f.clone();
    println!("{:?} {:?} {:?} {f:?}", f.next(), f.next(), f.next());
    println!("{:?}", copy.take(4).collect::<Vec<_>>());
    let mut v = vec![1, 2, 3].into_iter().fuse();
    println!("{:?}", v.size_hint());
    let w = v.clone();
    v.next();
    println!("{:?} {:?} {v:?}", v.size_hint(), w.collect::<Vec<_>>());
    println!("{:?} {:?}", v.by_ref().collect::<Vec<_>>(), v.size_hint());
    let mut flips = pair(Flip(0));
    println!("{flips:?}");
    let again = flips.clone();
    println!("{:?} {:?} {flips:?}", flips.next(), flips.next());
    println!("{again:?}");
    println!("{:?}", again.take(3).collect::<Vec<_>>());
    let mut loud = Loud("loud", 3).fuse();
    println!("{:?} {:?}", loud.next(), loud.next());
    println!("{:?}", loud.next());
    println!("ended");
    drop(loud);
    let mut steady = Steady(2).fuse();
    println!("{:?} {:?}", steady.next(), steady.next());
    println!("ended steady");
    drop(steady);
    for n in Loud("looped", 3).fuse() {
        println!("looped {n}");
    }
    println!("after loop");
    let total: u32 = Loud("summed", 4).fuse().map(|n| n * 10).sum();
    println!("summed {total}");
    println!("end");
}
