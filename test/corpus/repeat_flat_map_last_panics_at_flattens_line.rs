//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by a `flat_map`'s `last` of its inner iterator, `flatten.rs`'s line:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", std::iter::once(1u8).flat_map(|_| repeat(1u8)).last());
}
