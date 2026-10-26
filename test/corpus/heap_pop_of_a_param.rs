// `pop()` of a `BinaryHeap` of a type parameter's items, which could be
// `Option`s: its `Some` boxed, as a `Vec`'s is, so a `None` it holds is
// `Some(None)`, not an empty heap's `None`. As Prüfer's encoding pops its
// leaves in order.
use std::collections::BinaryHeap;

fn in_order<V: Ord + Copy>(items: &[V]) -> Vec<V> {
    let mut heap: BinaryHeap<V> = items.iter().copied().collect();
    let mut out = Vec::new();
    while let Some(v) = heap.pop() {
        out.push(v);
    }
    out
}

fn top<V: Ord + Copy>(items: &[V]) -> Option<V> {
    let mut heap = BinaryHeap::new();
    for &item in items {
        heap.push(item);
    }
    heap.pop()
}

fn main() {
    println!("{:?}", in_order(&[3, 1, 4, 1, 5]));
    println!("{:?}", in_order(&[Some(2), None, Some(7)]));
    println!(
        "{:?} {:?} {:?}",
        top(&[None::<u8>]),
        top::<u8>(&[]),
        top(&[(1, 'b'), (1, 'a')])
    );
    let mut plain = BinaryHeap::from(vec![2u8, 9, 4]);
    println!("{:?} {:?}", plain.pop(), plain.into_sorted_vec());
}
