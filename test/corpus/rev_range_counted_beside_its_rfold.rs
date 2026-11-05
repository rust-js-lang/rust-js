// A range's `rev()` the crate steps from its back by `rfold` somewhere is
// made knowing its back wherever it's made (ADR 0545, #702): what the
// range's own `count()`, `last()` and `nth()` of another of its `rev()`s
// read, its bounds, are its source's still, `$rev(..)`'s `of`, which threw
// where it had none (#721).
fn folded() -> u32 {
    (1..6u32).rev().rfold(0, |a, x| a * 10 + x)
}

fn folded_inclusive() -> u32 {
    (1..=5u32).rev().rfold(0, |a, x| a * 10 + x)
}

fn main() {
    println!("{}", folded());
    println!("{}", folded_inclusive());
    println!("{}", (1..6u32).rev().count());
    println!("{:?}", (1..6u32).rev().last());
    println!("{:?}", (1..6u32).rev().nth(1));
    println!("{}", (1..=5u32).rev().count());
    println!("{:?}", (1..=5u32).rev().last());
    println!("{:?}", (1..=5u32).rev().nth(1));
    let mut it = (1..6u32).rev();
    it.next();
    println!("{} {:?}", it.clone().count(), it.nth(1));
    let mut it = (1..=5u32).rev();
    it.next_back();
    println!("{:?} {:?}", it.clone().last(), it.nth(5));
}
