//@ run-fail: assertion failed: step != 0
// `step_by` asserts its step isn't 0 as it's given, before a single item
// is read: a step known only at run time is checked where it's given.
use std::hint::black_box;

fn every(items: &[i32], step: usize) -> Vec<i32> {
    items.iter().copied().step_by(step).collect()
}

fn main() {
    println!("{:?}", every(&[1, 2, 3, 4, 5], 2));
    println!("{:?}", (0..10).step_by(3).collect::<Vec<_>>());
    let empty = 0..0;
    let stepped = empty.step_by(black_box(0));
    println!("never {:?}", stepped.count());
}
