// A map of a range stepped from its end, given to generic code that asks
// its length, then steps it: std's compiled `Map` and `Rev` since the map
// and enumerate flip widened (#1681's change 2, step (b)), each step std's,
// where its hand-written form, made in place, was refused rather than read
// as its source's map stepped forward, `Some(2)` where Rust gives `Some(9)`
// (ADR 0987).
fn first<I: ExactSizeIterator<Item = u32>>(mut it: I) -> Option<u32> {
    println!("len {}", it.len());
    it.next()
}
fn main() {
    println!("{:?}", first((1..9u32).map(|x| x + 1).rev()));
    println!("{:?}", first((1..9u32).map(|x| x * 3).rev().skip(2)));
}
