// A `RefCell` of an object is the object (ADR 0863): of a `Vec`, a
// `HashMap`, a `HashSet` and a struct, borrowed, replaced, taken, swapped,
// replaced with, written through `get_mut`, cloned, compared, shown and
// made the default, each changing the one object every name of it sees,
// and its borrows counted where one is held while another may be asked.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Default, Clone, PartialEq)]
struct Totals {
    sum: i64,
    names: Vec<String>,
}

fn add(totals: &RefCell<Totals>, name: &str, n: i64) {
    let mut totals = totals.borrow_mut();
    totals.sum += n;
    totals.names.push(name.to_string());
}

fn main() {
    let shared = Rc::new(RefCell::new(vec![1, 2]));
    let other = Rc::clone(&shared);
    other.borrow_mut().push(3);
    let old = shared.replace(vec![9]);
    println!("{old:?} {:?} {:?}", shared.borrow(), other.borrow());
    let taken = other.take();
    println!("{taken:?} {} {}", shared.borrow().is_empty(), Rc::ptr_eq(&shared, &other));
    shared.borrow_mut().extend([4, 5]);
    let before = shared.replace_with(|v| v.iter().map(|n| n * 2).collect());
    println!("{before:?} {:?}", other.borrow());

    let left = RefCell::new(HashMap::from([("a", 1)]));
    let right = RefCell::new(HashMap::from([("b", 2)]));
    left.swap(&right);
    println!("{:?} {:?}", left.borrow().get("b"), right.borrow().get("a"));

    let mut seen = RefCell::new(HashSet::new());
    seen.get_mut().insert(1);
    seen.borrow_mut().insert(2);
    *seen.get_mut() = HashSet::from([7]);
    let mut items: Vec<_> = seen.borrow().iter().copied().collect();
    items.sort();
    println!("{items:?} {}", seen.borrow().contains(&7));

    let totals = RefCell::new(Totals::default());
    add(&totals, "a", 2);
    add(&totals, "b", 3);
    let copy = totals.clone();
    totals.borrow_mut().names.clear();
    println!("{:?} {:?} {}", totals, copy, totals == copy);
    *totals.borrow_mut() = Totals {
        sum: 1,
        names: vec!["c".to_string()],
    };
    let reading = totals.borrow();
    println!("{} {:?} {:?}", reading.sum, totals.try_borrow_mut().is_err(), totals);
    drop(reading);
    let fresh: RefCell<Vec<u8>> = RefCell::default();
    println!("{:?} {:?}", fresh.into_inner(), totals.into_inner().names);
}
