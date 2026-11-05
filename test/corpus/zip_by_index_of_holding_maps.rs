// A zip that reads its sides by index (ADR 0973) asks `b`'s length. Here
// both sides are `map()`s whose closures hold what drops, which MIR holds
// in variables so it can drop them. `b` is made knowing its length where
// it's made, as a map a bound asks its length of is (#775). Each side's
// closure drops where std's drops it: as the zip is dropped, `a`'s first.
// Run through `next`, `len`, `next_back`, a fold, a collect and a closure
// that panics part way.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn plus(g: &D, x: u32) -> u32 {
    g.0 + x
}

fn main() {
    let (v, w) = ([1, 2, 3], [16, 17, 18, 19]);
    let (ga, gb) = (D(3), D(4));
    let a = v.iter().map(move |x| plus(&ga, *x));
    let b = w.iter().map(move |x| plus(&gb, *x));
    let mut it = a.zip(b);
    println!("next {:?}", it.next());
    println!("len {}", it.len());
    println!("back {:?}", it.next_back());
    drop(it);

    let (ga, gb) = (D(5), D(6));
    let a = v.iter().map(move |x| plus(&ga, *x));
    let b = (0..10u32).map(move |x| plus(&gb, x));
    let sum = a.zip(b).fold(0, |s, (x, y)| s + x * y);
    println!("fold {sum}");

    let (ga, gb) = (D(7), D(8));
    let a = w.iter().map(move |x| plus(&ga, *x));
    let b = v.iter().map(move |x| plus(&gb, *x));
    let pairs: Vec<(u32, u32)> = a.zip(b).collect();
    println!("collect {pairs:?}");

    // A range's map that holds nothing that drops, made in place as it is.
    let pairs: Vec<(u32, u32)> = v.iter().map(|x| x + 1).zip((0..10u32).map(|x| x * 2)).collect();
    println!("collect {pairs:?}");

    let caught = catch_unwind(AssertUnwindSafe(|| {
        let (ga, gb) = (D(9), D(10));
        let a = v.iter().map(move |x| plus(&ga, *x));
        let b = w.iter().map(move |x| {
            assert!(*x != 17, "map");
            plus(&gb, *x)
        });
        for (x, y) in a.zip(b) {
            println!("for {x} {y}");
        }
    }));
    println!("panicked {}", caught.is_err());
}
