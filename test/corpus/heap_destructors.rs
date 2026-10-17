// A heap of values with destructors: `push`, `pop`, `append`, `from`,
// `into_sorted_vec` and `drain` move its items and drop none; what it gives
// back its holder drops, in the order the heap keeps them, and `retain`
// drops each it doesn't keep as it's met (ADRs 0098, 0333).
use std::collections::BinaryHeap;
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct O(u32);
impl Drop for O {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let mut h = BinaryHeap::new();
    h.push(O(3));
    h.push(O(9));
    h.push(O(5));
    let top = h.pop();
    println!("top {:?}", top.as_ref().map(|o| o.0));
    drop(top);
    let mut other = BinaryHeap::from(vec![O(1), O(7)]);
    h.append(&mut other);
    println!("len {} {}", h.len(), other.len());
    let drained: Vec<O> = h.drain().collect();
    println!("drained {}", drained.len());
    drop(drained);
    let sorted = BinaryHeap::from(vec![O(4), O(2), O(6)]).into_sorted_vec();
    println!("{:?}", sorted.iter().map(|o| o.0).collect::<Vec<_>>());
    let mut r = BinaryHeap::from(vec![O(20), O(25), O(21), O(30)]);
    r.retain(|o| {
        println!("keep {}?", o.0);
        o.0 % 5 == 0
    });
    println!("retained {:?}", r.peek().map(|o| o.0));
    drop(r);
    let mut k: BinaryHeap<O> = vec![O(11), O(12)].into();
    k.drain();
    println!("end {}", k.len());
}
