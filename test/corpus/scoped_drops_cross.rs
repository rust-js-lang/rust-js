//@ run-fail: gave up with second live
// A panic drops what's live as it unwinds, each value's `finally` where its
// scope ends (ADR 0437): of two values whose lives cross, the first dropped
// while the second is live, a panic after drops only the second, once.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn check(n: u32, what: &str) -> u32 {
    if n > 2 {
        panic!("gave up with {what}");
    }
    println!("checked {n}");
    n
}

fn main() {
    let first = Noisy("first");
    check(1, "first live");
    let second = Noisy("second");
    check(2, "both live");
    drop(first);
    check(1, "second live");
    let third = Noisy("third");
    drop(third);
    check(3, "second live");
    drop(second);
}
