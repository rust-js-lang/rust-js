//! Refused: this crate's impl of the library's trait asks the size of its
//! method's own `X`, which the library's generic code, given its
//! dictionary, has no way to give (#2285).
use dep::{Pick, pick_of};

struct Mine;

impl Pick for Mine {
    fn pick<X: Clone>(&self, x: X) -> usize {
        let _kept = x.clone();
        std::mem::size_of::<X>()
    }
}

pub fn main() {
    println!("{} {}", pick_of(&Mine), pick_of(&4u8));
}
