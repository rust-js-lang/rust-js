// A `Map` held is std's (ADR 0871, amended): of the crate's iterator,
// stepped from either end, counted, cloned with what its closure took, and
// shown; its `size_hint`, `fold` and `rfold`; held in a struct and given
// back from generic code. One made in place stays the JS iterator it is,
// cloned apart where an adapter it's part of is (ADR 0525), and cycled as
// either. Its `rfold` drops what its closure and the fold's took, the fold's
// first, as std's `map_fold` holds them.
use std::iter::Map;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Debug)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

impl ExactSizeIterator for Countdown {}

struct Doubled<I> {
    inner: Map<I, fn(u32) -> u32>,
}

fn double(x: u32) -> u32 {
    x * 2
}

fn doubled<I: Iterator<Item = u32>>(it: I) -> Map<I, fn(u32) -> u32> {
    it.map(double as fn(u32) -> u32)
}

fn last_of<I: DoubleEndedIterator<Item = u32>, F: FnMut(u32) -> u32>(mut m: Map<I, F>) -> Option<u32> {
    m.next_back()
}

#[derive(Clone)]
struct Holder<I> {
    it: I,
}

fn main() {
    let mut calls = 0;
    let mut m = Countdown(4).map(move |x| {
        calls += 1;
        x * 10 + calls
    });
    println!("{:?} {:?} {} {:?}", m.next(), m.next_back(), m.len(), m.size_hint());
    let c = m.clone();
    println!("{:?} {:?}", c.collect::<Vec<_>>(), m.collect::<Vec<_>>());

    let v = vec![1, 2, 3, 4, 5];
    let held = v.iter().map(|&x| x * x);
    println!("{}", held.clone().fold(0, |acc, x| acc * 10 + x));
    println!("{}", held.clone().rfold(0, |acc, x| acc * 100 + x));
    println!("{:?}", held.size_hint());

    let shown = Countdown(2).map(double as fn(u32) -> u32);
    println!("{shown:?}");

    let mut d = Doubled { inner: doubled(Countdown(3)) };
    println!("{:?} {:?} {}", d.inner.next(), d.inner.next_back(), d.inner.len());
    println!("{:?}", last_of(v.iter().copied().map(|x| x + 1)));

    let holder = Holder { it: (0..3).map(|x| x + 100) };
    let mut again = holder.clone();
    again.it.next();
    let cycled: Vec<_> = holder.clone().it.cycle().take(5).collect();
    println!("{cycled:?}");
    println!("{:?} {:?}", again.it.collect::<Vec<_>>(), holder.it.collect::<Vec<_>>());

    // Made in place: part of a filter cloned apart, and cycled.
    let evens = [1, 2, 3, 4, 5, 6].iter().map(|x| x * 3).filter(|x| x % 2 == 0);
    let mut first = evens.clone();
    first.next();
    println!("{:?} {:?}", first.collect::<Vec<_>>(), evens.collect::<Vec<_>>());
    println!("{:?}", (1..3).map(|x| x * 3).cycle().take(5).collect::<Vec<_>>());
    // Read by a `rev()` that's held: std's struct, as its `rev()` is.
    let cubes = v.iter().map(|x| x * x * x);
    let mut back = cubes.rev();
    println!("{:?} {:?} {}", back.next_back(), back.next(), back.len());

    let by = Loud("map's");
    let held = Holder {
        it: (1..4).map(move |x| {
            let _ = &by;
            x * 2
        }),
    };
    let into = Loud("fold's");
    let total = held.it.rfold(0, move |a, x| {
        let _ = &into;
        a * 10 + x
    });
    println!("{total}");
}
