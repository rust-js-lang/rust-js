//! A consumer's generic function giving its own `I` to a library trait's
//! methods whose `where` asks `I`'s `DoubleEndedIterator`, changing nothing
//! in place, and a concrete iterator's, whose dictionary it makes.
use dep::Ends;

fn trimmed<I: DoubleEndedIterator<Item = i32> + Clone>(it: I) -> (Vec<i32>, Option<(i32, i32)>) {
    (it.clone().all_but_last(1), it.first_and_last())
}

pub fn main() {
    println!("{:?}", trimmed([5, 6, 7].iter().copied()));
    println!("{:?}", trimmed(1..4));
    println!("{:?}", trimmed(std::iter::once(9)));
    println!("{:?}", [1, 2, 3].iter().copied().all_but_last(2));
    println!("{:?}", (0..0).first_and_last());
}
