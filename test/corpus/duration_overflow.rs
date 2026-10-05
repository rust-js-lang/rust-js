//@ run-fail: overflow in Duration::new
// A `Duration` past `u64::MAX` seconds panics as std's does.
use std::time::Duration;

fn main() {
    println!("{:?}", Duration::new(u64::MAX, 999_999_999));
    let d = Duration::new(u64::MAX, 1_000_000_000);
    println!("{d:?}");
}
