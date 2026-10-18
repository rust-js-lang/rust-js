//@ run-fail: overflow when subtracting durations
// A `Duration`'s `-=` past zero panics as its `-` does (ADR 0188).
use std::time::Duration;

fn main() {
    let mut left = Duration::from_millis(1);
    println!("before");
    left -= Duration::from_secs(1);
    println!("{left:?}");
}
