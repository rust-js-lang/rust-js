//@ run-fail: capacity overflow
// A slice's `repeat` of a count whose items are past `usize::MAX` panics
// where std's `[T]::repeat` multiplies them, its `checked_mul(n)
// .expect("capacity overflow")`, `slice.rs`'s line, before it asks any
// room, whose overflow is raw_vec's line (#2488).
fn main() {
    println!("{:?}", [1u8, 2].repeat(3));
    let n = usize::MAX / 2 + 1;
    println!("{}", [1u8, 2].repeat(n).len());
}
