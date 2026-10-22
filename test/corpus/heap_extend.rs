// `extend` of a `BinaryHeap` pushes each item, then rebuilds its order
// from the first of them, as std's does: by any `Extend`, of an array, a
// slice, another heap, a set or a mapped iterator.
use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

fn add<E: Extend<i32>>(e: &mut E) {
    e.extend([7, 70]);
}

fn main() {
    let mut heap = BinaryHeap::from([1]);
    heap.extend(vec![100, -5]);
    heap.extend(&[8, 2]);
    println!("{:?} {:?}", heap.peek(), heap);
    heap.extend(BinaryHeap::from([50, 3]));
    heap.extend(BTreeSet::from([60]));
    add(&mut heap);
    println!("{:?}", heap.clone().into_sorted_vec());

    let mut low: BinaryHeap<Reverse<i32>> = BinaryHeap::new();
    low.extend([5, 1, 9].iter().map(|&x| Reverse(x)));
    println!("{:?}", low.pop());
}
