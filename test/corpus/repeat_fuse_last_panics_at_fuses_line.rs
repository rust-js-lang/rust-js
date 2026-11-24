//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by `fuse`'s `last`, `fuse.rs`'s line:
// the nearest frame that doesn't track its caller (#2488).
use std::iter::repeat;

#[derive(Clone)]
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    println!("start");
    println!("{:?}", repeat(L(5)).fuse().last().map(|l| l.0));
}
