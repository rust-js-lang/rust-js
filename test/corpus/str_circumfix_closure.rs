//@ compile-fail: rust-js does not support calling `core::str::<impl str>::strip_circumfix` yet
// `strip_circumfix` of a closure as a pattern: only a `&str` or a `char` one.
fn main() {
    println!("{:?}", "xay".strip_circumfix(|c: char| c == 'x', 'y'));
}
