// A `Vec`, a `BinaryHeap` and a `VecDeque` cleared, nothing grown or
// reserved, keep what they had room for (ADR 0419): `clear` is among what
// may shorten one and keeps its capacity (#1109).
use std::collections::{BinaryHeap, VecDeque};

fn main() {
    let mut items = vec![1u32, 2, 3];
    items.clear();
    println!("{} {}", items.len(), items.capacity());
    let mut heap = BinaryHeap::from(vec![4u32, 1, 3]);
    heap.clear();
    println!("{} {}", heap.len(), heap.capacity());
    let mut queue = VecDeque::from(vec![1u32, 2]);
    queue.clear();
    println!("{} {}", queue.len(), queue.capacity());
}
