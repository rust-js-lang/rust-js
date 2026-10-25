// A set's `iter().for_each(..)` is the set's own `forEach`, `set.forEach((x)
// => ..)`, each item given once, in the set's order, as Rust's steps it
// (ADR 0873): of numbers, of text, of tuples a `$KeySet` keeps, of objects
// by themselves, and of one a `RefCell` lends. What each sees is told in
// an order that doesn't hang on the set's, its sum or its items sorted. A
// map's values are no set: `Array.from(map.values())`'s, as they were.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Queue {
    values: RefCell<Vec<u32>>,
}

impl PartialEq for Queue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Queue {}

impl Hash for Queue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

fn sum(set: &HashSet<u32>) -> u32 {
    let mut total = 0;
    set.iter().for_each(|n| total += n);
    total
}

fn joined(set: &HashSet<String>) -> String {
    let mut seen = Vec::new();
    set.iter().for_each(|word| seen.push(word.to_uppercase()));
    seen.sort();
    seen.join(",")
}

fn pairs(set: &HashSet<(u32, u32)>) -> u32 {
    let mut total = 0;
    set.iter().for_each(|(a, b)| total += a * b);
    total
}

fn values_sum(map: &HashMap<&str, u32>) -> u32 {
    let mut total = 0;
    map.values().for_each(|n| total += n);
    total
}

fn push_all(queues: &Rc<RefCell<HashSet<&'static Queue>>>, value: u32) {
    queues.borrow().iter().for_each(|queue| {
        queue.values.borrow_mut().push(value);
    });
}

fn main() {
    let numbers: HashSet<u32> = [3, 1, 4, 1, 5].into_iter().collect();
    println!("{}", sum(&numbers));
    let words: HashSet<String> = ["b", "a", "b"].into_iter().map(String::from).collect();
    println!("{}", joined(&words));
    let tuples: HashSet<(u32, u32)> = [(1, 2), (3, 4), (1, 2)].into_iter().collect();
    println!("{}", pairs(&tuples));
    let a: &'static Queue = Box::leak(Box::new(Queue { values: RefCell::new(Vec::new()) }));
    let b: &'static Queue = Box::leak(Box::new(Queue { values: RefCell::new(vec![9]) }));
    let queues = Rc::new(RefCell::new(HashSet::new()));
    queues.borrow_mut().insert(a);
    queues.borrow_mut().insert(b);
    queues.borrow_mut().insert(a);
    push_all(&queues, 7);
    push_all(&queues, 8);
    println!("{:?} {:?}", a.values.borrow(), b.values.borrow());
    // Nothing given: the closure runs once for each.
    let mut count = 0;
    numbers.iter().for_each(|_| count += 1);
    println!("{count}");
    let map: HashMap<&str, u32> = [("a", 2), ("b", 5)].into_iter().collect();
    println!("{}", values_sum(&map));
}
