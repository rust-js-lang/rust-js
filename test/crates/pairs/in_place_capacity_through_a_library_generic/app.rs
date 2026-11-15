//! A collect in place through a library's generic function the consumer
//! compiles here: its adapters are std's structs, linked to their source,
//! so the capacity is the source's buffer's, as native Rust's (#1934).
use dep::doubled;

pub fn main() {
    let mut kept = Vec::with_capacity(7);
    kept.extend([4, 5]);
    let d: Vec<u32> = doubled(kept.into_iter()).collect();
    println!("{d:?} {}", d.capacity());
}
