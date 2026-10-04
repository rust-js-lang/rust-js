//@ run-fail: min > max, or either was NaN. min = NaN, max = 1.0
// A float's `clamp` with a NaN bound panics, showing each bound as `{:?}`
// does.
fn main() {
    println!("{}", 0.5f64.clamp(0.0, 1.0));
    let low = f64::NAN;
    println!("{}", 0.5f64.clamp(low, 1.0));
}
