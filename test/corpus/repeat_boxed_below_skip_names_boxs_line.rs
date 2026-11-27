//@ run-fail: iterator is infinite
// A `repeat()`'s own `last`, which panics naming its caller as std's
// tracks it, through a `Box` below a `skip`: `boxed/iter.rs`'s line, the
// box's `last` the nearest frame that doesn't track its caller, which the
// box's lend gives as it passes the step on (#2488, #2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", Box::new(repeat(1u8)).skip(1).last());
}
