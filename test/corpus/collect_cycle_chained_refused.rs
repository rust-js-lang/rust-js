//@ compile-fail: a `cycle()` chained with what has an end
// `collect()` into a `Vec` of a `cycle()` chained with what has an end
// panics `capacity overflow` where the cycle has items. Where it has none,
// as here, it collects what follows: its hint's lower bound after its first
// item is `usize::MAX` only where that item is the cycle's, which only what
// it holds as it runs says (ADR 0940, amended).
fn main() {
    let none: [usize; 0] = [];
    let cycled = none.iter().cycle();
    let all: Vec<&usize> = cycled.chain([9].iter()).collect();
    println!("{all:?}");
}
