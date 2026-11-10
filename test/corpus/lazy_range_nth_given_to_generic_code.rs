// A range's numbers made as they're stepped, `$lazyRange`, given to generic
// code that asks its `nth` billions in: its `nth` is std's `Range`'s, of its
// bounds, never each number stepped, as native Rust's isn't (#231, #1109).
fn far<I: Iterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.nth(2_000_000_000)
}

fn main() {
    println!("{:?}", far(0..u32::MAX));
    println!("{:?}", far(5..u32::MAX));
    println!("{:?}", far(10..2_000_000_005));
}
