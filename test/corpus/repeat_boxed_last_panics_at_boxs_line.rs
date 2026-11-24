//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by a `Box`'s `last`, `boxed/iter.rs`'s line:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", Box::new(repeat(1u8)).last());
}
