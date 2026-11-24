//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by a chain's `last`, which gives `Iterator::last` to `Option::and_then`, the `FnOnce` shim's line:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", [1u8].into_iter().chain(repeat(1u8)).last());
}
