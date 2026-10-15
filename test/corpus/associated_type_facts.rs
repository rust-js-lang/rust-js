// `size_of` of an associated type in generic code, `<A as Array>::Item`,
// smallvec's: a fact its trait's dictionary carries, `AArray.ItemSize`,
// each impl's of its own type, `T`'s, given its factory (ADR 0145). Asked
// directly, its alignment, or by a callee it's given to, its size.
use std::mem;

trait Array {
    type Item;
    fn size() -> usize;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
    fn size() -> usize {
        N
    }
}

fn item_align<A: Array>() -> usize {
    mem::align_of::<A::Item>() * A::size()
}

fn layout<T>(n: usize) -> usize {
    mem::size_of::<T>() * n
}

fn grown<A: Array>(n: usize) -> usize {
    layout::<A::Item>(n)
}

fn main() {
    println!("{} {} {}", item_align::<[u16; 4]>(), grown::<[u64; 2]>(3), grown::<[(); 5]>(2));
}
