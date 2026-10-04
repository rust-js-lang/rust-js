//@ run-fail: assertion failed: n <= self.len()
// A `VecDeque`'s assertion names the count `n`, where a slice's says `k`.
use std::collections::VecDeque;

fn main() {
    let mut q: VecDeque<char> = "ab".chars().collect();
    q.rotate_right(1);
    println!("{:?}", q);
    q.rotate_right(3);
}
