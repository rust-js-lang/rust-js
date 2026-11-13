//! A crate that reads a `Vec`'s capacity, so compiles the library's impl
//! itself, whose dictionary's entries give each default the facts it's
//! given of its items' iterator: an `Option`'s at most one item, so a
//! hint's upper bound (#1613).
use dep::Ext;

pub fn main() {
    let v: Vec<u32> = vec![[1u32, 2, 3], [4, 5, 6]].into_iter().flat();
    println!("{} {}", v.len(), v.capacity());
    println!("{:?}", vec![Some(1u32), None, Some(3)].into_iter().hint());
    println!("{:?}", vec![[1u32, 2], [3, 4]].into_iter().hint());
    println!("{:?}", vec![vec![1u32], vec![2]].into_iter().hint());
}
