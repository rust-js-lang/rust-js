//! Filters of a deque's items past a `skip`, given the library in an array.
use std::collections::VecDeque;

fn odd(x: &u32) -> bool {
    x % 2 == 1
}

pub fn main() {
    let n: VecDeque<u32> = (1..7).collect();
    let filters = [
        n.iter().skip(1).copied().filter(odd),
        n.iter().skip(4).copied().filter(odd),
    ];
    println!("{:?}", dep::backs(filters));
}
