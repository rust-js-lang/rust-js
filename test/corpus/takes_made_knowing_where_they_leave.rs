// A concrete `take` generic code asks the hint of is made knowing it,
// `$taking`, in each function whose value of it may leave for the one that
// asks: given back, given to a function of the crate's, captured by a closure
// that asks, or given to it where it's made; one made and stepped where it's
// made is JS's own adapter there, `v.slice(0, 2)`, where once any asked it
// was `$taking` in the whole crate (#1333).
use std::iter::Take;
use std::slice::Iter;

/// Generic code that asks what it's given its hint.
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

/// Made and stepped where it's made: no one asks its hint.
fn summed(v: &[u32]) -> u32 {
    v.iter().take(2).sum()
}

/// Made and given back, to a caller that asks.
fn first_two(v: &[u32]) -> Take<Iter<'_, u32>> {
    v.iter().take(2)
}

/// Made and given to a function of the crate's, which asks.
fn given(v: &[u32]) -> (usize, Option<usize>) {
    passed(v.iter().take(2))
}

fn passed(it: Take<Iter<'_, u32>>) -> (usize, Option<usize>) {
    hint(it)
}

/// Made and captured by a closure that asks.
fn captured(v: &[u32]) -> (usize, Option<usize>) {
    let it = v.iter().take(2);
    let ask = move || hint(it);
    ask()
}

fn main() {
    let v = [1, 2, 3, 4];
    println!("{:?}", hint(v.iter().take(2)));
    println!("{}", summed(&v));
    println!("{:?}", hint(first_two(&v)));
    println!("{:?}", given(&v));
    println!("{:?}", captured(&v));
}
