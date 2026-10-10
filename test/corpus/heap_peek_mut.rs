// A `BinaryHeap`'s `peek_mut`: its top changed in place, and sifted down as
// the guard drops, only if it was written; and `PeekMut::pop`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;

/// Says each time it's compared.
#[derive(Debug, PartialEq, Eq)]
struct Loud(u32);

impl Ord for Loud {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("cmp {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn main() {
    let mut heap: BinaryHeap<i32> = [3, 8, 5, 1].into_iter().collect();
    if let Some(mut top) = heap.peek_mut() {
        *top = 0;
    }
    println!("{:?}", heap.as_slice());
    {
        let mut top = heap.peek_mut().unwrap();
        *top -= 10;
        println!("{} {:?}", *top, top);
    }
    println!("{:?}", heap.as_slice());
    let top = heap.peek_mut().unwrap();
    println!("popped {}", PeekMut::pop(top));
    println!("{:?}", heap.as_slice());
    let mut loud: BinaryHeap<Loud> = BinaryHeap::new();
    loud.push(Loud(2));
    loud.push(Loud(7));
    println!("read");
    if let Some(top) = loud.peek_mut() {
        println!("top {}", top.0);
    }
    println!("write");
    if let Some(mut top) = loud.peek_mut() {
        top.0 = 1;
    }
    println!("{:?} {:?}", loud.as_slice(), BinaryHeap::<i32>::new().peek_mut().is_none());
}
