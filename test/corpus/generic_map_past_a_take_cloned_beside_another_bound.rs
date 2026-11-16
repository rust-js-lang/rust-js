//@ library-differs
// `pub` generic code, whose callers JS or a consumer may be too, that
// cycles the `Map<I, F>` it makes in place, beside a bound of a type
// parameter the `Map` doesn't hold, `L: Display`: what the `Map` stands for
// is cloned each round, by its bounds as any caller's may be, and a bound
// none of its parameters' values decide, `L`'s, is one it may meet, so it's
// made cloned apart, each round from where it started (ADR 0525, #955,
// #1109). Its `map` is past a `fuse()`, std's struct wherever it's made,
// which the held flip never takes out: the flip makes a generic chain's
// `map` std's compiled struct only from its source outward, so it leaves
// this one the hand-written form, cloned apart as before (#1780).
use std::fmt::Display;

pub fn cycled<I, F, L>(items: I, f: F, label: L) -> Vec<i32>
where
    I: Iterator<Item = i32> + Clone,
    F: FnMut(i32) -> i32 + Clone,
    L: Display,
{
    println!("{label}");
    items.fuse().map(f).cycle().take(5).collect()
}

fn main() {
    println!("{:?}", cycled(1..3, |x| x * x, "squares"));
    println!("{:?}", cycled(vec![4, 5, 6].into_iter(), |x| -x, 2));
}
