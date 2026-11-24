//@ run-fail: capacity overflow
// A `str`'s `repeat` past `usize::MAX` bytes, uncaught: std's hook names
// `slice.rs`'s line, its `checked_mul`'s `expect`, where JS's own `repeat`
// threw a `RangeError` at `<unknown>` (#2487, #2511).
fn main() {
    println!("start");
    let s = "ab".repeat(usize::MAX);
    println!("{}", s.len());
}
