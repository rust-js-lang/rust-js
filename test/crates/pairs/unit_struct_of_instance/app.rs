//! A consumer that calls the library's generic default, which it compiles
//! itself from the library's MIR, where `..` and a unit struct are
//! constants, not values made (#474).
pub fn main() {
    let letters = dep::Letters(b'a');
    println!("{}", dep::Engine::encode(&letters, b"xyz"));
    println!("{}", dep::Engine::encode(&letters, vec![1, 2]));
}
