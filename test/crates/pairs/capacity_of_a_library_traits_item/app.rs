//! The library's growth of its trait's associated type's items, each of its
//! impls, none of this crate's own, so the size its manifest says its
//! dictionaries carry is all this crate's copy is given (#1260).
use dep::gathered;

pub fn main() {
    println!("{} {}", gathered(&dep::Bytes).capacity(), gathered(&dep::Wide).capacity());
}
