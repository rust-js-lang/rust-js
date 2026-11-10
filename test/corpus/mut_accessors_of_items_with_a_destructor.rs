// std's methods that lend a `&mut` into a collection, or move an item in
// or out of it, of items with a destructor: each drops what native Rust
// drops, a value written over through the `&mut` then, one moved out none,
// and `insert_mut`'s item as its index past the end panics (#1274).

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn quiet<T>(f: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(f)).ok()
}

fn main() {
    let mut v = vec![D(1), D(2)];
    println!("{:?}", quiet(|| { v.insert_mut(5, D(9)).0 += 1; }));
    v.insert_mut(1, D(3)).0 += 10;
    v.push_mut(D(4)).0 += 100;
    println!("{v:?}");
    let mut q: VecDeque<D> = VecDeque::from([D(1), D(2), D(3)]);
    println!("{:?}", q.swap_remove_front(9));
    println!("{:?}", q.swap_remove_back(0));
    q.push_front_mut(D(7)).0 += 1;
    q.push_back_mut(D(8)).0 += 1;
    let (a, b) = q.as_mut_slices();
    for d in a.iter_mut().chain(b.iter_mut()) {
        d.0 *= 2;
    }
    println!("{q:?}");
    let mut h = BinaryHeap::from([D(3), D(5), D(1)]);
    if let Some(mut top) = h.peek_mut() {
        *top = D(0);
    }
    println!("{:?}", h.into_sorted_vec());
    let mut m: BTreeMap<u32, D> = (1..4).map(|i| (i, D(i))).collect();
    if let Some(mut e) = m.first_entry() {
        println!("{:?}", e.insert(D(40)));
    }
    for (_, d) in m.range_mut(2..) {
        d.0 += 50;
    }
    println!("{:?} {:?}", m.pop_first(), m.pop_last());
    println!("{m:?}");
    let mut s: BTreeSet<D> = (1..4).map(D).collect();
    println!("{:?} {:?} {}", s.pop_first(), s.pop_last(), s.len());
    let mut items = [D(1), D(2), D(3), D(4), D(5)];
    if let Some(c) = items.first_chunk_mut::<2>() {
        c[0] = D(10);
    }
    for c in items.rchunks_exact_mut(2) {
        c.swap(0, 1);
    }
    println!("{items:?}");
}
