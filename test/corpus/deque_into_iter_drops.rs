// A `VecDeque`'s and a `BinaryHeap`'s `into_iter()` own their items, as a
// `Vec`'s does: behind an adapter, what it doesn't give is dropped as the
// iterator is (ADR 0098, #203).
use std::collections::{BinaryHeap, VecDeque};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn deque(items: &[u32]) -> VecDeque<Loud> {
    items.iter().map(|&n| Loud(n)).collect()
}

fn main() {
    let counted = deque(&[0, 1]).into_iter().fuse().count();
    println!("counted {counted}");

    let taken: Vec<Loud> = deque(&[2, 3, 9, 4]).into_iter().take_while(|l| l.0 < 5).collect();
    println!("taken {}", taken.len());
    drop(taken);

    let mut peeked = deque(&[5, 6, 7]).into_iter().peekable();
    let first = peeked.next();
    drop(peeked);
    println!("first {}", first.map_or(0, |l| l.0));

    let skipped: Vec<Loud> = deque(&[1, 8, 2]).into_iter().skip_while(|l| l.0 < 5).collect();
    println!("skipped to {}", skipped.len());
    drop(skipped);

    let heap: BinaryHeap<Loud> = [Loud(10), Loud(11)].into_iter().collect();
    let some = heap.into_iter().take(1).count();
    println!("heap {some}");
}
