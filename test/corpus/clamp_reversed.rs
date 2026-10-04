//@ run-fail: min > max. min = 5, max = 1
// An integer's `clamp` with its bounds the wrong way round panics, showing
// each.
fn main() {
    let (low, high) = (5, 1);
    println!("{}", 3.clamp(high, low));
    println!("{}", 3.clamp(low, high));
}
