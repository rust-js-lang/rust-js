//! A consumer that calls a library trait's generic default of the
//! library's own impl, whose dictionary the library exports: called through
//! it, and nothing of the library's compiled here is written (#483).
use dep::Shout;

pub fn main() {
    println!("{}", dep::Dog.shout("!"));
    println!("{}", dep::Dog.shout(String::from("?")));
}
