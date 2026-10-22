//@ run-fail: out of bounds access
// A `VecDeque`'s place past its end, written, panics with its own message,
// as a read of it does.
use std::collections::VecDeque;
use std::hint::black_box;

fn main() {
    let mut queue = VecDeque::from([1, 2]);
    queue[black_box(0)] = 7;
    println!("{queue:?}");
    queue[black_box(4)] = 9;
}
