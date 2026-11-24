//@ run-fail: assertion failed: min <= max
// A `char`'s own `clamp` of bounds the wrong way round panics showing
// them, `min > max. min = 'z', max = 'a'`, at its caller's line. A
// `&char`'s is `Ord`'s default, as `Ord for &A` writes no `clamp`:
// `assertion failed: min <= max`, at std's own line (#2506).
use std::panic;

fn said(caught: Result<char, Box<dyn std::any::Any + Send>>) -> String {
    match caught {
        Ok(c) => format!("ok {c}"),
        Err(payload) => payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default(),
    }
}

fn main() {
    println!("{}", *(&'m').clamp(&'a', &'k'));
    let by_ref = panic::catch_unwind(|| *(&'m').clamp(&'z', &'a'));
    let by_value = panic::catch_unwind(|| 'm'.clamp('z', 'a'));
    println!("{} | {}", said(by_ref), said(by_value));
    println!("{}", *(&'q').clamp(&'r', &'b'));
}
