//@ ignore-rust-js: THIR's lowering refuses `next_back()` of a `RangeInclusive` a variable keeps
// `next_back()` of an inclusive range a variable keeps: its end, moved down,
// to its start, and then nothing.
fn main() {
    let mut inclusive = 1..=3;
    let a = inclusive.next_back();
    let b = inclusive.next_back();
    let c = inclusive.next();
    println!("{a:?} {b:?} {c:?} {:?}", inclusive.next_back());
}
