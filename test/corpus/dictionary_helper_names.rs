//@ library-differs
// A trait's dictionary of the crate's own is cached in a `$` name, as the
// runtime's helpers are named: one named as a helper the module imports,
// a generic `Duration`'s `Add` and std's `Duration` added, is given apart
// from it.
use std::ops::Add;
use std::time::Duration as StdDuration;

#[derive(Clone, Copy, Debug)]
struct Duration<T>(T);

impl<T: Add<Output = T>> Add for Duration<T> {
    type Output = Duration<T>;
    fn add(self, other: Duration<T>) -> Duration<T> {
        Duration(self.0 + other.0)
    }
}

fn total<T: Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

fn main() {
    let d = total(Duration(1u32), Duration(2));
    let s = StdDuration::from_secs(1) + StdDuration::from_millis(500);
    println!("{d:?} {s:?}");
}
