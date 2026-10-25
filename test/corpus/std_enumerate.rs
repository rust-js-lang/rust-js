// An `Enumerate` held is std's (ADR 0871, amended): of the crate's
// iterator, stepped from either end, its index from the back its length's;
// counted, cloned and shown; its `nth`, `nth_back` and `rfold`; held in a
// struct and given back from generic code. One a variable holds that only
// `next()`, a `for` loop or `drop` steps reads as one stepped where it's
// made, the recognizers' `entries()`.
use std::iter::Enumerate;

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

struct Numbered<I> {
    inner: Enumerate<I>,
}

fn numbered<I: Iterator>(it: I) -> Enumerate<I> {
    it.enumerate()
}

fn last_of<I: DoubleEndedIterator + ExactSizeIterator>(mut e: Enumerate<I>) -> Option<(usize, I::Item)> {
    e.next_back()
}

fn main() {
    let mut e = Countdown(4).enumerate();
    println!("{:?} {:?} {}", e.next(), e.next_back(), e.len());
    let c = e.clone();
    println!("{c:?}");
    println!("{:?}", c.collect::<Vec<_>>());

    let v = vec![10, 20, 30, 40, 50];
    let mut held = v.iter().enumerate();
    println!("{:?} {:?} {:?}", held.nth(1), held.nth_back(1), held.len());
    println!("{:?}", held.clone().rfold(0, |acc, (i, &x)| acc * 100 + i * 10 + x as usize));
    println!("{held:?}");

    let mut n = Numbered { inner: numbered(Countdown(3)) };
    println!("{:?} {:?}", n.inner.next(), n.inner.next_back());
    println!("{:?}", last_of(numbered(v.iter().copied())));

    // Stepped through a variable alone.
    let mut chars = "abc".chars().enumerate();
    while let Some((i, c)) = chars.next() {
        print!("{i}{c} ");
    }
    println!();
    let mut words = ["x", "y", "z"].into_iter().enumerate();
    for (i, w) in words.by_ref() {
        if i == 1 {
            break;
        }
        print!("{w}");
    }
    println!(" {:?}", words.next());
    // Of a `&mut` to the crate's iterator, its length the iterator's own.
    let mut counting = Countdown(3);
    let mut by_mut = (&mut counting).enumerate();
    println!("{:?} {}", by_mut.next_back(), by_mut.len());
    // Lent where its own impl steps it, from its back: std's struct.
    let mut ends = v.iter().enumerate();
    println!("{:?} {:?}", ends.by_ref().next_back(), ends.next());
}
