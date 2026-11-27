//@ run-fail: iterator is infinite
// A `repeat()`'s own `last`, which panics naming its caller as std's
// tracks it, through a `Box` below a `copied()` below a `skip`:
// `boxed/iter.rs`'s line, the box's `last` the nearest of the frames that
// don't track their caller, as each of the box's lend and `$copied` gives
// its own as it passes the step on, the innermost last (#2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", Box::new(repeat(&1u8)).copied().skip(1).last());
}
