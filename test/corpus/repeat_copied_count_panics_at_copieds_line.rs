//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by `copied`'s `count`, `copied.rs`'s line:
// the nearest frame that doesn't track its caller (#2488). Its items
// first, each as it is, `$copied`'s of what it's given no copy (#2507).
use std::iter::repeat;

fn main() {
    println!("{:?}", repeat(&1u8).copied().take(2).collect::<Vec<_>>());
    println!("{}", repeat(&1u8).copied().count());
}
