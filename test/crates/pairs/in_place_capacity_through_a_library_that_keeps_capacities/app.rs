//! A collect in place through a function of a library that keeps
//! capacities: the source's buffer's capacity, as native Rust's, as the
//! library's iterator is linked to its source where it's made (#1934).
use dep::{doubled, room};

pub fn main() {
    let mut kept = Vec::with_capacity(7);
    kept.extend([4, 5]);
    let d: Vec<u32> = doubled(kept.into_iter()).collect();
    println!("{d:?} {} {}", d.capacity(), room(&d));
}
