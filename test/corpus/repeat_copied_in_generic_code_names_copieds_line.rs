//@ run-fail: iterator is infinite
// A `repeat()`'s own `last`, which panics naming its caller as std's
// tracks it, through a `copied()` generic code makes of what it's given:
// `copied.rs`'s line, `Copied::last` the nearest frame that doesn't track
// its caller, given where its `last` is called, as JS has none of it
// (#2488, #2507).
use std::iter::repeat;

fn last_of<'a, I: Iterator<Item = &'a u8>>(it: I) -> Option<u8> {
    it.copied().last()
}

fn main() {
    println!("start");
    println!("{:?}", last_of(repeat(&1u8)));
}
