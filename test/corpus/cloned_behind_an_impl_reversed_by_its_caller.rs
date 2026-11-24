//@ library-differs
// A clone of a deque's items a function gives behind an `impl
// DoubleEndedIterator`, alone or held in a type of its own, reversed by its
// caller's `rev()`: what the function makes is made knowing its back where
// it's made, as its caller can't make it so. std's `Cloned` is kept as its
// meaning, not std's struct, which steps from its back by its own methods,
// and a deque's items have no form that steps their clones from the back
// (#1979's `held-adapter-not-back-stepped`, `held-adapter-opaque` and
// `given-impl-not-back-stepped`).
use std::collections::VecDeque;

#[derive(Debug)]
struct Noisy(u32);

impl Clone for Noisy {
    fn clone(&self) -> Noisy {
        println!("clone {}", self.0);
        Noisy(self.0)
    }
}

enum Two<L, R> {
    One(L),
    Other(R),
}

impl<L: Iterator, R: Iterator<Item = L::Item>> Iterator for Two<L, R> {
    type Item = L::Item;

    fn next(&mut self) -> Option<L::Item> {
        match self {
            Two::One(l) => l.next(),
            Two::Other(r) => r.next(),
        }
    }
}

impl<L: DoubleEndedIterator, R: DoubleEndedIterator<Item = L::Item>> DoubleEndedIterator for Two<L, R> {
    fn next_back(&mut self) -> Option<L::Item> {
        match self {
            Two::One(l) => l.next_back(),
            Two::Other(r) => r.next_back(),
        }
    }
}

fn items(d: &VecDeque<Noisy>) -> impl DoubleEndedIterator<Item = Noisy> + '_ {
    d.iter().cloned()
}

fn either(
    first: bool,
    d: &VecDeque<Noisy>,
) -> Two<impl DoubleEndedIterator<Item = Noisy> + '_, impl DoubleEndedIterator<Item = Noisy> + '_> {
    match first {
        true => Two::One(d.iter().cloned()),
        false => Two::Other(d.range(1..).cloned()),
    }
}

fn main() {
    let d: VecDeque<Noisy> = (1..4).map(Noisy).collect();
    let back: Vec<Noisy> = items(&d).rev().collect();
    println!("{back:?}");
    println!("{:?}", items(&d).rev().nth(1));
    let held: Vec<Noisy> = either(true, &d).rev().collect();
    let other: Vec<Noisy> = either(false, &d).rev().collect();
    println!("{held:?} {other:?}");
}
