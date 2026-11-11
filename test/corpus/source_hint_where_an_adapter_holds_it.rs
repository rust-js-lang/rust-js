// What an adapter holds asks its source's hint, std's: an explicit `Take`
// of a `successors`, `(1, Some(n))` while it holds one, `(0, Some(0))` once
// it's done or made of `None`; of a `repeat_with`, `(n, Some(n))`; each as
// a struct's field, stepped, chained and collected, and under a
// `peekable()`. Each printed `(0, Some(n))`, as though its source might
// give none (#1357's P0).
use std::iter::{Peekable, RepeatWith, Successors, Take};

type Next = fn(&u32) -> Option<u32>;
type Make = fn() -> u32;

struct Held {
    counting: Take<Successors<u32, Next>>,
    ones: Take<RepeatWith<Make>>,
    none: Take<Successors<u32, Next>>,
    peeked: Peekable<Successors<u32, Next>>,
}

fn step(n: &u32) -> Option<u32> {
    (*n < 4).then_some(n + 1)
}

fn one() -> u32 {
    1
}

fn main() {
    let mut held = Held {
        counting: std::iter::successors(Some(1), step as Next).take(3),
        ones: std::iter::repeat_with(one as Make).take(3),
        none: std::iter::successors(None, step as Next).take(3),
        peeked: std::iter::successors(Some(3), step as Next).peekable(),
    };
    println!("{:?}", held.counting.size_hint());
    println!("{:?}", held.ones.size_hint());
    println!("{:?}", held.none.size_hint());
    println!("{:?}", held.peeked.size_hint());
    println!("{:?}", held.peeked.peek());
    println!("{:?}", held.peeked.size_hint());

    held.counting.next();
    held.ones.next();
    println!("{:?} {:?}", held.counting.size_hint(), held.ones.size_hint());
    println!("{:?}", held.peeked.by_ref().collect::<Vec<_>>());
    println!("{:?}", held.peeked.size_hint());

    let chained = held.counting.chain(held.ones);
    println!("{:?}", chained.size_hint());
    let all: Vec<u32> = chained.collect();
    println!("{all:?}");

    let taken = std::iter::repeat_with(|| 5u8).take(4).peekable();
    println!("{:?}", taken.size_hint());
}
