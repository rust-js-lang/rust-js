//@ run-fail: iterator is infinite
// A `repeat()`'s own `last`, which panics naming its caller as std's
// tracks it, through a `Box` of a `Box` below a `skip`: `boxed/iter.rs`'s
// line, the inner box's `last` the nearest frame that doesn't track its
// caller (#2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", Box::new(Box::new(repeat(1u8))).skip(1).last());
}
