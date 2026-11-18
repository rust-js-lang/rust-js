// `repeat(x)` holds `x` and gives a clone of it at each step, and its
// `nth` and `advance_by` step past nothing; dropped, it drops `x`; its
// `last` and `count` panic, as it has no end, stepping nothing. In the build of std's bodies it is std's compiled
// `Repeat`, made by std's compiled `repeat`, whose `last` and `count` panic
// with a text alone, the `fmt::Arguments::from_str` rustc's inliner put in
// std's optimized body, called again (#1569).
//@ run-fail: iterator is infinite
use std::iter::repeat;

#[derive(Debug)]
struct L(u32);

impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 100)
    }
}

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let taken: Vec<L> = repeat(L(7)).take(2).collect();
    println!("{taken:?}");
    drop(taken);
    let mut it = repeat(L(8));
    println!("{:?}", it.next());
    println!("{:?}", it.nth(3));
    drop(it);
    let skipped: Vec<u32> = repeat(L(4)).skip(5).take(1).map(|l| l.0).collect();
    println!("{skipped:?}");
    let paired: Vec<(u8, L)> = (1..3).zip(repeat(L(9))).collect();
    println!("{paired:?}");
    drop(paired);
    println!("{:?}", repeat(2u8).take(3).collect::<Vec<_>>());
    let counted = std::panic::catch_unwind(|| repeat(5u8).count());
    println!("count panicked: {}", counted.is_err());
    repeat(L(6)).last();
}
