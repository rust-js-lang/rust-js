// A set of counted `Rc`s of a type equal only to itself (ADR 0864,
// amended): each `Rc` is the `{ value, strong, weak }` every clone of it
// shares, one allocation's, so a JS `Set` of them keys each fiber once.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Fiber {
    lanes: u32,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

fn main() {
    let a = Rc::new(Fiber { lanes: 1 });
    let b = Rc::new(Fiber { lanes: 2 });
    let mut updaters = HashSet::new();
    updaters.insert(Rc::clone(&a));
    updaters.insert(Rc::clone(&a));
    updaters.insert(Rc::clone(&b));
    println!("{} {} {}", updaters.len(), Rc::strong_count(&a), Rc::strong_count(&b));
    updaters.remove(&a);
    println!(
        "{} {} {} {}",
        updaters.len(),
        Rc::strong_count(&a),
        updaters.contains(&b),
        a.lanes + b.lanes
    );
}
