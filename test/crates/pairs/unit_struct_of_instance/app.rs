//! A consumer that calls the library's generic default, which it compiles
//! itself from the library's MIR, where `..` and a unit struct are
//! constants, not values made (#474): of its own impl, whose dictionary
//! calls the one it compiles, and of the library's, whose dictionary the
//! library exports (#483).
use dep::Engine;

struct Upper;

impl Engine for Upper {
    fn letter(&self) -> u8 {
        b'A'
    }
}

pub fn main() {
    let letters = dep::Letters(b'a');
    println!("{}", dep::Engine::encode(&letters, b"xyz"));
    println!("{}", dep::Engine::encode(&letters, vec![1, 2]));
    println!("{}", Upper.encode(b"pq"));
}
