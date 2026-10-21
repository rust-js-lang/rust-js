// A `Peekable` is std's own (ADR 0552): its `next`,
// `next_back`, `count`, `nth`, `last`, `fold`, `next_if` and `next_if_eq`,
// its clone and its `Debug` are std's bodies, compiled from the MIR the
// toolchain ships into `@rust-js/std`, over `{ iter, peeked }`. What it
// steps is stepped through its dictionary: a slice's iterator here, and a
// range, in generic code and in concrete.
use std::iter::Peekable;

fn first_two<I: Iterator<Item = u32>>(it: I) -> (Option<u32>, Option<u32>, Option<u32>) {
    let mut p = it.peekable();
    let a = p.next();
    let b = p.next_if(|&x| x % 2 == 0);
    let c = p.next();
    (a, b, c)
}

fn from_back<I: DoubleEndedIterator<Item = u32>>(it: I) -> Vec<u32> {
    let mut p = it.peekable();
    let mut out = vec![p.next_back().unwrap_or(0)];
    out.push(p.next().unwrap_or(0));
    out.push(p.next_back().unwrap_or(0));
    out
}

fn summed<I: Iterator<Item = u32> + Clone>(it: I) -> (u32, usize, Option<u32>, Option<u32>)
where
    Peekable<I>: Clone,
{
    let p = it.peekable();
    let copy = p.clone();
    let total = p.fold(0, |acc, x| acc + x);
    let mut again = copy.clone();
    let third = again.nth(2);
    (total, copy.count(), third, again.last())
}

fn main() {
    let v = vec![1, 2, 3, 4];
    println!("{:?}", first_two(v.iter().copied()));
    println!("{:?}", first_two(1..5));
    println!("{:?}", first_two(std::iter::empty()));
    println!("{:?}", from_back(v.iter().copied()));
    println!("{:?}", from_back(10..13));
    println!("{:?}", summed(v.iter().copied()));
    println!("{:?}", summed(1..4));
    let mut p = v.clone().into_iter().peekable();
    println!("{:?} {:?}", p.next_if_eq(&1), p.next_if_eq(&1));
    println!("{:?}", p);
    let mut r = (5..8).peekable();
    println!("{:?} {:?} {:?}", r.next(), r.next_back(), r);
    println!("{}", r.count());
}
