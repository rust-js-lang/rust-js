// A range's `rev()` that generic code folds from its back is made knowing
// its back wherever the crate makes one, `$rev(..)`, as each value of a
// type is: what the range's own `count()`, `last()` and `nth()` of another
// of its `rev()`s read, its bounds, are still its source's, through the
// `$rev` (#1226).
fn digits_back<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    it.rfold(0, |a, x| a * 10 + x)
}

fn countdown(it: impl DoubleEndedIterator<Item = u32>) -> impl DoubleEndedIterator<Item = u32> {
    it
}

fn main() {
    println!("{}", digits_back((1..6u32).rev()));
    println!("{}", digits_back(countdown((1..=5u32).rev())));
    println!("{}", (1..6u32).rev().count());
    println!("{:?}", (1..6u32).rev().last());
    println!("{:?}", (1..6u32).rev().nth(1));
    println!("{}", (1..=5u32).rev().count());
    println!("{:?}", (1..=5u32).rev().last());
    println!("{:?}", (1..=5u32).rev().nth(1));
}
