//@ run-fail: iterator is infinite
//@ located-otherwise: #2507 a frame JS compiles away below a runtime adapter
// A `repeat()`'s own `count`, which panics naming its caller as std's
// tracks it, through a `copied()` below a `skip`: Rust names `copied.rs`,
// `Copied::count` the nearest frame that doesn't track its caller; rust-js
// names the `skip`'s, as nothing of the `copied()` stands between them in
// JS (#2488, #2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{}", repeat(&1u8).copied().skip(1).count());
}
