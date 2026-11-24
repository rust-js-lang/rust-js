//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by `peekable`'s `count` of nothing peeked, `peekable.rs`'s line of that arm:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{}", repeat(7u32).peekable().count());
}
