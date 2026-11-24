//! A library's generic functions that collect, or ask the hint of, what
//! their consumer gives them, which the consumer compiles itself (ADR
//! 0914): what its body asks is the consumer's to see as its own body's
//! is, so a chain of two takes of a set's items it's given is made knowing
//! its hint, and std's `Vec` its capacity, by `next`, room for the hint's
//! lower bound and one more after the first item (#2456).

pub fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

/// Two levels down, a private function's collect.
pub fn collected_within<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    within(it)
}

fn within<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

pub fn hint<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}
