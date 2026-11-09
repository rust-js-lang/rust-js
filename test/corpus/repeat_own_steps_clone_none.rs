// What std's `Repeat` and `RepeatN` write themselves, not std's defaults of
// their `next`, which clone each item they pass: `Repeat::advance_by`, which
// makes no item, so `take(n).count()` clones none (#1124), and `nth`, one
// clone; `RepeatN`'s `nth`, `last`, the value itself, and `count`, its
// length; asked through `take`, `skip` and `step_by`, held, and in generic
// code, each clone and drop printed, against native Rust.
use std::iter::{repeat, repeat_n};
struct L(u32);
impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 1)
    }
}
impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn counted<T: Clone>(x: T, n: usize) -> usize {
    repeat(x).take(n).count()
}

fn skipped<T: Clone>(x: T) -> usize {
    repeat_n(x, 3).skip(1).count()
}

fn main() {
    println!("{}", repeat(L(0)).take(3).count());
    println!("{:?}", repeat(L(10)).take(5).nth(2).map(|l| l.0));
    println!("{:?}", repeat(L(20)).skip(3).next().map(|l| l.0));
    println!("{:?}", repeat(L(30)).nth(4).map(|l| l.0));
    println!("{}", repeat(L(40)).take(4).step_by(2).count());
    println!("{}", repeat(L(50)).take(3).skip(1).count());
    println!("{}", repeat_n(L(60), 3).count());
    println!("{:?}", repeat_n(L(70), 4).nth(2).map(|l| l.0));
    println!("{:?}", repeat_n(L(80), 3).nth(5).map(|l| l.0));
    println!("{:?}", repeat_n(L(90), 3).last().map(|l| l.0));
    println!("{:?}", repeat_n(L(100), 3).skip(2).next().map(|l| l.0));
    println!("{}", repeat_n(L(110), 0).count());
    let mut held = repeat(L(120)).take(5);
    held.nth(1);
    println!("{}", held.count());
    let mut held_n = repeat_n(L(130), 5);
    held_n.next();
    println!("{}", held_n.count());
    println!("{}", counted(L(140), 3));
    println!("{}", skipped(L(150)));
    println!("{}", repeat(L(160)).take(3).fold(0, |n, _| n + 1));
    for l in repeat_n(L(170), 2) {
        println!("got {}", l.0);
    }
}
