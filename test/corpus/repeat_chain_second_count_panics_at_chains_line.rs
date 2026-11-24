//@ run-fail: iterator is infinite
// A `repeat()`'s own step, which panics naming its caller as std's
// tracks it, reached by a chain's `count` of its second side, `chain.rs`'s line of it:
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
    println!("{}", [L(6)].into_iter().chain(repeat(L(7))).count());
}
