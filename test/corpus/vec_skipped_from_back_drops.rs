// A `Vec`'s `IntoIter` skipped from its back drops what it passes over as
// one slice, front to back, as std's `advance_back_by` drops it (#234):
// its `nth_back`, a `rev()`'s `nth`, `skip` and `step_by`, and a
// `step_by` of one. A `VecDeque`'s does too; a `Drain`'s drops each as
// it's passed, from the back.
use std::collections::VecDeque;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(from: u32, n: u32) -> Vec<Loud> {
    (from..from + n).map(Loud).collect()
}

fn main() {
    for loud in louds(1, 4).into_iter().rev().step_by(3) {
        println!("got {}", loud.0);
    }
    println!("{:?}", louds(5, 3).into_iter().rev().skip(1).nth(2).map(|l| l.0));
    let mut it = louds(8, 4).into_iter();
    println!("{:?}", it.nth_back(2).map(|l| l.0));
    println!("{:?}", louds(12, 4).into_iter().rev().nth(2).map(|l| l.0));
    for loud in louds(28, 4).into_iter().rev().skip(2) {
        println!("got {}", loud.0);
    }
    for loud in louds(16, 3).into_iter().rev().step_by(1).step_by(3) {
        println!("got {}", loud.0);
    }
    let deque: VecDeque<Loud> = louds(20, 4).into();
    println!("{:?}", deque.into_iter().rev().nth(2).map(|l| l.0));
    let mut drained = louds(24, 4);
    println!("{:?}", drained.drain(..).nth_back(2).map(|l| l.0));
    println!("end");
}
