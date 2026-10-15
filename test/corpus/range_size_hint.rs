// A range's `size_hint()` is how many numbers are left, exactly, as both
// bounds, and its `len()` the same: none once stepping has passed its end
// (ADR 0129).
fn main() {
    let mut long = 0..=255u8;
    let mut short = 42..=42u8;
    println!("{:?} {:?}", long.size_hint(), short.size_hint());
    long.next();
    short.next();
    println!("{:?} {:?} {}", long.size_hint(), short.size_hint(), short.is_empty());
    println!("{} {}", long.len(), short.len());
    let mut r = 3..7;
    r.next();
    println!("{:?}", r.size_hint());
    let start = 5;
    let backwards = start..2;
    println!("{} {:?}", backwards.len(), backwards.size_hint());
}
