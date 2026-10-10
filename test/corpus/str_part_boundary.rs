//@ run-fail: end byte index 1 is not a char boundary; it is inside 'é' (bytes 0..2 of string)
// A part of a string, `&mut s[a..b]`, is checked as it's made, before it's
// used.
fn main() {
    let mut s = String::from("éa");
    let part = &mut s[..1];
    println!("made");
    part.make_ascii_uppercase();
}
