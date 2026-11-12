// A `BinaryHeap` keeps the buffer of the `Vec` it's made of: collected,
// it's `Vec::from_iter`'s, grown by its iterator's hint, then
// `BinaryHeap::from`; and `into_sorted_vec()` sorts that buffer in place.
// So its capacity is the `Vec`'s, not its length (#1531).
use std::collections::BinaryHeap;

fn main() {
    // Collected from iterators that aren't `TrustedLen`.
    let h: BinaryHeap<u32> = (0..100u32).filter(|x| x % 3 == 0).collect();
    println!("filter {} {}", h.len(), h.capacity());
    let mut items = [1u32, 2, 3].iter().flat_map(|&x| vec![x; 12]);
    items.next();
    let h: BinaryHeap<u32> = items.collect();
    println!("flat_map {} {}", h.len(), h.capacity());
    let h: BinaryHeap<u32> = (0..10u32).chain((0..30u32).filter(|x| x % 4 == 0)).collect();
    println!("chain {} {}", h.len(), h.capacity());
    let h: BinaryHeap<u32> = (0..7u32).collect();
    println!("range {} {}", h.len(), h.capacity());

    // Made of a `Vec` with room to spare, and given back.
    let mut v = Vec::with_capacity(10);
    v.push(3u32);
    v.push(1);
    let h = BinaryHeap::from(v);
    println!("from {} {}", h.len(), h.capacity());
    let back = h.into_vec();
    println!("into_vec {} {}", back.len(), back.capacity());
    let mut w = Vec::with_capacity(12);
    w.extend([7u32, 2, 9]);
    let sorted = BinaryHeap::from(w).into_sorted_vec();
    println!("into_sorted_vec {:?} {}", sorted, sorted.capacity());

    // Of an array, as long as it is.
    let h = BinaryHeap::from([5u32, 1, 9]);
    println!("array {} {}", h.len(), h.capacity());

    // Grown after.
    let mut h: BinaryHeap<u32> = (0..20u32).filter(|x| x % 5 == 0).collect();
    h.push(100);
    h.extend((0..40u32).filter(|x| x % 7 == 0));
    println!("grown {} {} {:?}", h.len(), h.capacity(), h.peek());
}
