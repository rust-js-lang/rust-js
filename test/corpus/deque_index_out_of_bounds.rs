//@ run-fail: out of bounds access
// A `VecDeque`'s index past its end panics with its own message, `out of
// bounds access`, not a slice's `index out of bounds`.
use std::collections::VecDeque;
use std::hint::black_box;

fn main() {
    let mut queue = VecDeque::from([1, 2]);
    queue[1] = 5;
    println!("{queue:?} {}", queue[0]);
    let items = vec![3];
    println!("{}", items[black_box(0)]);
    println!("{}", queue[black_box(3)]);
}
