// A `BinaryHeap`'s and a `VecDeque`'s capacity is its array's, as a
// `Vec`'s is (ADR 0419): at least what `with_capacity` and `reserve` gave,
// kept where it's popped or cleared. And a heap of `()`, whose items are
// all `Equal`, pushed, counted and peeked, not popped.
use std::collections::{BinaryHeap, VecDeque};

fn main() {
    let mut heap = BinaryHeap::with_capacity(10);
    println!("{} {}", heap.capacity() >= 10, heap.len());
    heap.push(3);
    heap.push(7);
    heap.reserve(20);
    println!("{} {:?}", heap.capacity() >= 22, heap.pop());
    heap.clear();
    println!("{} {}", heap.capacity() >= 22, heap.is_empty());
    heap.shrink_to_fit();
    println!("{}", heap.capacity() >= heap.len());

    let mut grown = BinaryHeap::new();
    for n in 0..30 {
        grown.push(n);
    }
    grown.clear();
    println!("{}", grown.capacity() >= 30);

    let mut queue: VecDeque<u8> = VecDeque::with_capacity(8);
    println!("{}", queue.capacity() >= 8);
    for n in 0..20 {
        queue.push_back(n);
    }
    queue.pop_front();
    queue.clear();
    println!("{}", queue.capacity() >= 20);

    let mut units = BinaryHeap::new();
    for _ in 0..4 {
        units.push(());
    }
    println!("{} {} {:?}", units.len(), units.iter().count(), units.peek());
}
