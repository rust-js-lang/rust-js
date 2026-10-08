//@ compile-fail: rust-js does not support calling `core::str::<impl str>::make_ascii_uppercase` yet
// A leaked `String`'s `&mut str` is the string (ADR 0238), as nothing
// writes a `str` in place: a write through it is refused.
fn main() {
    let shout = format!("ab").leak();
    shout.make_ascii_uppercase();
    println!("{shout}");
}
