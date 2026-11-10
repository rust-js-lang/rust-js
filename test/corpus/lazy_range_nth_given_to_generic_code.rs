// A range's numbers made as they're stepped, `$lazyRange`, given to generic
// code that asks its `nth` billions in, or a `skip` of them, whose first
// step is its source's `nth`: of its bounds, its `advance_by`, never each
// number stepped, as native Rust's isn't (#231, #1109, #1310).
fn far<I: Iterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.nth(2_000_000_000)
}

fn skipped<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    it.skip(3_000_000_000).next()
}

fn main() {
    println!("{:?}", far(0..u32::MAX));
    println!("{:?}", far(5..u32::MAX));
    println!("{:?}", far(10..2_000_000_005));
    println!("{:?}", skipped(5..u32::MAX));
    println!("{:?}", skipped(10..20));
}
