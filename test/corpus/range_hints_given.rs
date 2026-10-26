// A range given to generic code knows how many numbers it has left, as
// std's does: its `size_hint` exact, an inclusive one's too, and its `len`
// where it's an `ExactSizeIterator` (ADR 0918); and a `Skip`'s `len`, std's
// default of its exact `size_hint`, which a `take()` over it asks from its
// back, as a `step_by()` asks its range's.
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn len<I: ExactSizeIterator>(it: I) -> usize {
    it.len()
}

fn main() {
    println!("{:?} {:?}", hint(1..=5u8), hint(3..=2u16));
    println!("{:?} {:?}", hint(2..7), hint((0..10).step_by(3)));
    let mut r = 1..=4u8;
    r.next();
    println!("{:?} {}", hint(r.clone()), len(r));
    println!("{} {}", len(0..9), len([1, 2, 3].iter().skip(1)));
    println!("{:?}", (1..=10u8).skip(7).rev().collect::<Vec<_>>());
    println!("{:?}", [1, 2, 3, 4].iter().skip(1).take(2).rev().collect::<Vec<_>>());
    println!("{:?}", (1..=10u8).step_by(3).rev().collect::<Vec<_>>());
}
