//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by a chain's `count` of its first side, `chain.rs`'s line of it:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{}", repeat(1u8).chain([1u8]).count());
}
