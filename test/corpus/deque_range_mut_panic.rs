//@ run-fail: range end index 9 out of range for slice of length 3
// A `VecDeque`'s `range_mut` past its end panics as std's does.
use std::collections::VecDeque;

fn main() {
    let mut d: VecDeque<i32> = [1, 2, 3].into();
    for x in d.range_mut(1..9) {
        *x += 1;
    }
}
