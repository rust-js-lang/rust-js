//@ run-fail: index out of bounds
// `VecDeque::insert` past the end panics with its own message, not `Vec`'s.
use std::collections::VecDeque;

fn main() {
    let mut queue: VecDeque<i32> = VecDeque::new();
    queue.push_back(1);
    let at = queue.len() + 1;
    queue.insert(at, 5);
}
