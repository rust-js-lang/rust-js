//@ run-fail: start byte index 1 is not a char boundary; it is inside 'é' (bytes 0..2 of string)
// A range of a `&mut str` is checked as `&s[a..b]` is: inside a `char`, it panics.
fn main() {
    let mut s = String::from("éa");
    s[1..].make_ascii_uppercase();
    println!("{s}");
}
