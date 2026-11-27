//@ run-fail: iterator is infinite
// A `repeat()`'s own `count`, which panics naming its caller as std's
// tracks it, through a `copied()` below a `skip`: `copied.rs`'s line,
// `Copied::count` the nearest frame that doesn't track its caller, which
// `$copied` gives as it passes the step on (#2488, #2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{}", repeat(&1u8).copied().skip(1).count());
}
