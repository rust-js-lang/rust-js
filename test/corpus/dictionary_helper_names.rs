// A trait's dictionary of the crate's own is cached in a `$` name, as the
// runtime's helpers are named: one named as a helper the module imports,
// `Duration`'s `Add` and std's `Duration` added, is given apart from it.
use std::ops::Add;
use std::time::Duration as StdDuration;

#[derive(Clone, Copy, Debug)]
struct Duration(u32);

impl Add for Duration {
    type Output = Duration;
    fn add(self, other: Duration) -> Duration {
        Duration(self.0 + other.0)
    }
}

fn total<T: Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

fn main() {
    let d = total(Duration(1), Duration(2));
    let s = StdDuration::from_secs(1) + StdDuration::from_millis(500);
    println!("{d:?} {s:?}");
}
