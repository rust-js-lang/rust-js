//@ run-fail: attempt to calculate the remainder with a divisor of zero
// A remainder a closure writes to what it captured, `last = 17 % d`,
// panics where its `%` is, as native Rust's does, not where the
// assignment starts (#2366, found by generated programs, seed 12467702).
fn main() {
    let mut last = 0i32;
    let mut rest = |d: i32| {
        last = 17 % d;
        last
    };
    println!("{}", rest(5));
    println!("{}", rest(0));
}
