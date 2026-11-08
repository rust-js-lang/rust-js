//! A library's iterators stepped from their back here, and adapters made
//! here that its generic functions step from theirs (#1024).
use dep::{Bag, back, filtered, last_mapped};

pub fn main() {
    println!("{:?}", filtered().next_back());
    println!("{:?}", filtered().rev().collect::<Vec<_>>());
    let b = Bag(vec![1, 2, 3, 4, 6]);
    println!("{:?}", b.odd_squares().rev().collect::<Vec<_>>());
    let mut e = b.evens();
    println!("{:?} {:?} {:?}", e.next_back(), e.next(), e.collect::<Vec<_>>());
    let v = vec![1u32, 2, 3, 4, 5];
    println!("{:?}", back(v.iter().copied().filter(|x| x % 2 == 1)));
    println!("{:?}", last_mapped((1..4u32).map(|x| x * 10)));
}
