//@ run-fail: iterator is infinite
// A `repeat()`'s own `count`, which panics naming its caller as std's
// tracks it, through a `copied()` a chain's first side: `copied.rs`'s
// line, `Copied::count` the nearest frame that doesn't track its caller,
// nearer than the chain's (#2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{}", repeat(&1u8).copied().chain([2u8]).count());
}
