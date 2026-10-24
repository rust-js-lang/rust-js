// An `Enumerate` a struct holds, as itertools' `Positions` does: its
// source and its count kept, so what's asked of it later, its size hint,
// its `Debug`, and its back where the impl's bounds give it, is Rust's
// (ADR 0542).
use std::fmt;
use std::iter::Enumerate;

struct Positions<I, F> {
    iter: Enumerate<I>,
    f: F,
}

fn positions<I: Iterator, F: FnMut(I::Item) -> bool>(iter: I, f: F) -> Positions<I, F> {
    Positions { iter: iter.enumerate(), f }
}

impl<I: Iterator, F: FnMut(I::Item) -> bool> Iterator for Positions<I, F> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let f = &mut self.f;
        self.iter.find_map(|(count, val)| f(val).then_some(count))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.iter.size_hint().1)
    }
}

impl<I, F> DoubleEndedIterator for Positions<I, F>
where
    I: DoubleEndedIterator + ExactSizeIterator,
    F: FnMut(I::Item) -> bool,
{
    fn next_back(&mut self) -> Option<usize> {
        let f = &mut self.f;
        self.iter.by_ref().rev().find_map(|(count, val)| f(val).then_some(count))
    }

    fn rfold<B, G: FnMut(B, usize) -> B>(self, init: B, mut func: G) -> B {
        let mut f = self.f;
        self.iter.rfold(init, |mut acc, (count, val)| {
            if f(val) {
                acc = func(acc, count);
            }
            acc
        })
    }
}

impl<I: fmt::Debug, F> fmt::Debug for Positions<I, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Positions").field("iter", &self.iter).finish()
    }
}

// A map of a `&mut` to generic code's iterator, stepped from its back, as
// either's `Either` iterates: its impl's `&mut R: DoubleEndedIterator` asks
// `R`'s.
fn last_doubled<R: DoubleEndedIterator<Item = u32>>(items: &mut R) -> Option<u32> {
    items.map(|x| x * 2).rfold(None, |last, x| last.or(Some(x)))
}

// A struct holding an `Enumerate` of a range: made knowing its back and
// length, which its back asks.
struct Numbered {
    iter: Enumerate<std::ops::Range<u32>>,
}

// One holding generic code's iterator, given as it is, mapped through a
// `&mut` to it and stepped from its back.
struct Wrap<R> {
    inner: R,
}

impl<R: DoubleEndedIterator<Item = u32>> Wrap<R> {
    fn last_tripled(&mut self) -> Option<u32> {
        self.inner.by_ref().map(|x| x * 3).rfold(None, |last, x| last.or(Some(x)))
    }
}

fn main() {
    let mut even = positions(vec![1, 2, 4, 5, 6].into_iter(), |x| x % 2 == 0);
    println!("{:?}", even.size_hint());
    println!("{:?} {:?}", even.next(), even.next_back());
    println!("{even:?}");
    println!("{even:#?}");
    println!("{:?}", even.rfold(Vec::new(), |mut all, i| {
        all.push(i);
        all
    }));
    // An array's, which the crate's types holding an `Enumerate` makes
    // explicit too, from its back, as itertools' combinations step theirs.
    let indices = [7, 8, 9];
    for (i, x) in indices.iter().enumerate().rev() {
        print!("{i}:{x} ");
    }
    println!();
    let mut rest = vec![1u32, 2, 3].into_iter();
    println!("{:?} {:?}", last_doubled(&mut rest), rest.next());
    let mut numbered = Numbered { iter: (5..8).enumerate() };
    println!("{:?} {:?}", numbered.iter.next_back(), numbered.iter.next());
    println!("{:?}", numbered.iter.next());
    let mut wrap = Wrap { inner: vec![4u32, 5].into_iter() };
    println!("{:?} {:?}", wrap.last_tripled(), wrap.inner.next());
    // Of a range, stepped as it is by its dictionaries.
    let mut counted = Wrap { inner: 1u32..4 };
    println!("{:?} {:?}", counted.last_tripled(), counted.inner.next());
    // A range given as it is, stepped from its back by its dictionaries.
    let mut big = positions(1..20u32, |x| x > 16);
    println!("{:?} {:?} {:?}", big.next(), big.next_back(), big.next());
}
