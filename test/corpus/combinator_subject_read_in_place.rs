// A combinator's subject Rust reads once, a field of what a function is
// given, is read once: in place where the JS reads it once anyway, at the
// root of its chain, `q.items.push(3)` and `$swapRemove(q.items, 0)`, not
// held in a `const` first (#1908); and held where it's read twice,
// `q.step.map_or(..)`'s.
use std::cell::RefCell;

struct Queue {
    items: RefCell<Vec<u32>>,
    step: Option<f64>,
}

fn add(q: &Queue, item: u32) {
    q.items.borrow_mut().push(item);
}

fn take_first(q: &Queue) -> u32 {
    q.items.borrow_mut().swap_remove(0)
}

fn doubled(q: &Queue) -> f64 {
    q.step.map_or(0.0, |s| s * 2.0)
}

fn main() {
    let q = Queue { items: RefCell::new(vec![1, 2]), step: Some(1.5) };
    add(&q, 3);
    add(&q, 4);
    let first = take_first(&q);
    println!("{first} {:?} {}", q.items.borrow(), doubled(&q));
    let none = Queue { items: RefCell::new(vec![7]), step: None };
    println!("{} {}", take_first(&none), doubled(&none));
}
