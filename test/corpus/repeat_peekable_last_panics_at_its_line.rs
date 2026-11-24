//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by `peekable`'s `last`, `peekable.rs`'s line:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", repeat(7u32).peekable().last());
}
