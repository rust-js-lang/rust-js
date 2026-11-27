//@ run-fail: iterator is infinite
// A `repeat()`'s own `last`, which panics naming its caller as std's
// tracks it, through a `Box` generic code makes of what it's given, below
// a `skip`: `boxed/iter.rs`'s line, the box's `last` the nearest frame
// that doesn't track its caller, which the box's lend gives as it passes
// the step on, whatever its caller gives (#2507).
use std::iter::repeat;

fn last_of<I: Iterator<Item = u8>>(it: I) -> Option<u8> {
    Box::new(it).skip(1).last()
}

fn main() {
    println!("start");
    println!("{:?}", last_of(repeat(1u8)));
}
