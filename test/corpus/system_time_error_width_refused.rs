//@ compile-fail: options for a `SystemTimeError`'s
// A placeholder's width of a `SystemTimeError` is its `Duration`'s, inside
// it, `SystemTimeError(                1.5s)`, not the whole text's, which
// rust-js padded (ADR 0447): refused, as other options of it are.
use std::time::{Duration, UNIX_EPOCH};

fn main() {
    let error = UNIX_EPOCH.duration_since(UNIX_EPOCH + Duration::from_millis(1500)).unwrap_err();
    println!("[{error:>20?}]");
}
