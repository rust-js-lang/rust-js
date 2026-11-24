//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by `skip`'s `count`, `skip.rs`'s line:
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
    println!("{}", repeat(L(3)).skip(2).count());
}
