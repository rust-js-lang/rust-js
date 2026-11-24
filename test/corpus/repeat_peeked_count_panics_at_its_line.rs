//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by `peekable`'s `count` after a `peek`, `peekable.rs`'s line of its other arm:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

fn main() {
    println!("start");
    let mut p = repeat(7u32).peekable();
    println!("{:?}", p.peek());
    println!("{}", p.count());
}
