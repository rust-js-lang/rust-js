//@ run-fail: attempt to calculate the remainder with a divisor of zero
// `wrapping_rem` wraps `MIN % -1`, not a zero divisor, which panics as `%`
// does.
fn main() {
    println!("{}", i32::MIN.wrapping_rem(-1));
    let zero = 0;
    println!("{}", 7i32.wrapping_rem(zero));
}
