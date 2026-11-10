// What a function gives behind an `impl Trait` is what its value is: std's
// struct of an adapter it makes, `(0..n).rev()`, which its callers step by
// std's own methods, never the knowing form a type stepped from its back
// elsewhere is made, so `(1..6).rev()` in `main` is as it'd be anyway;
// and what it gives back of what it's given, `at(it)`, the value its caller
// made and gave it, of its form, whatever the type it hides says (#1226,
// the user's decision (A)).
use std::fmt::Debug;

fn down(n: u32) -> impl DoubleEndedIterator<Item = u32> + Clone + Debug {
    (0..n).rev()
}

fn at<I: Iterator + DoubleEndedIterator>(it: I) -> impl Iterator<Item = I::Item> + DoubleEndedIterator {
    it
}

fn main() {
    println!("{}", down(4).rfold(0, |a, x| a * 10 + x));
    println!("{:?}", (1..6u32).rev().last());
    let d = down(3);
    println!("{:?} {:?}", d.clone().collect::<Vec<_>>(), d.rev().next());
    let v = vec![1u32, 2, 3];
    println!("{}", at(v.iter().map(|n| n + 1)).rfold(0, |a, b| a * 10 + b));
    println!("{:?}", at(at(v.into_iter()).map(|n| n * 2)).next_back());
}
