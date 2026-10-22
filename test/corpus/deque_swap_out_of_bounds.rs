//@ run-fail: assertion failed: j < self.len()
// A `VecDeque`'s `swap` asserts each index, as std's does, where a slice's
// panics as an index out of bounds does.
use std::collections::VecDeque;

fn main() {
    let mut d: VecDeque<i32> = VecDeque::from(vec![1, 2, 3]);
    d.swap(0, 2);
    println!("{d:?}");
    let j = d.len();
    d.swap(0, j);
}
