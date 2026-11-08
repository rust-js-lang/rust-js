//! The tag is in the string after `+=`, as Rust writes it.
use dep::Tag;

pub fn main() {
    let mut s = String::from("x");
    s += Tag("b");
    s += "!";
    println!("{s}");
}
