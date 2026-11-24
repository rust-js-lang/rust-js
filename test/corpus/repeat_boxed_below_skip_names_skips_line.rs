//@ run-fail: iterator is infinite
//@ located-otherwise: #2507 a frame JS compiles away below a runtime adapter
// A `repeat()`'s own `last`, which panics naming its caller as std's
// tracks it, through a `Box` below a `skip`: Rust names `boxed/iter.rs`,
// the box's `last` the nearest frame that doesn't track its caller;
// rust-js names the `skip`'s, as nothing of the box stands between them
// in JS (#2488, #2507).
use std::iter::repeat;

fn main() {
    println!("start");
    println!("{:?}", Box::new(repeat(1u8)).skip(1).last());
}
