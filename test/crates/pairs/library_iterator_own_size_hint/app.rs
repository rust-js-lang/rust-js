//! The library's own `size_hint` called, then asked through its generic
//! `hint`, which once read std's `(0, None)`.
pub fn main() {
    println!("{:?}", dep::Countdown(3).size_hint());
    println!("{:?}", dep::hint(&dep::Countdown(5)));
}
