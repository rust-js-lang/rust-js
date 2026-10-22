// A closure given through generic code to `retain_mut`, `VecDeque`'s and
// `HashMap::retain`, changes each item it keeps, of numbers and of structs:
// generic code gives it a handle, which the closure made at the call takes.
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
struct Counter {
    n: u32,
}

fn keep_some<T, F: FnMut(&mut T) -> bool>(items: &mut Vec<T>, f: F) {
    items.retain_mut(f);
}

fn keep_queued<T, F: FnMut(&mut T) -> bool>(queue: &mut VecDeque<T>, f: F) {
    queue.retain_mut(f);
}

fn keep_entries<K, V, F: FnMut(&K, &mut V) -> bool>(map: &mut HashMap<K, V>, f: F) {
    map.retain(f);
}

fn main() {
    let mut numbers = vec![1, 2, 3, 4];
    keep_some(&mut numbers, |n| {
        *n *= 3;
        *n % 2 == 0
    });
    let mut counters = vec![Counter { n: 1 }, Counter { n: 2 }];
    keep_some(&mut counters, |c| {
        c.n += 10;
        c.n > 11
    });
    println!("{numbers:?} {counters:?}");
    let mut queue = VecDeque::from([1, 2, 3]);
    keep_queued(&mut queue, |x| {
        *x += 1;
        *x > 2
    });
    let mut map = HashMap::from([(1, 10), (2, 20)]);
    keep_entries(&mut map, |k, v| {
        *v += 1;
        *k == 2
    });
    println!("{queue:?} {map:?}");
}
