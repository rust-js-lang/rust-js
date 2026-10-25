// React's `Transition` keeps the fibers it updated in a set, each by itself
// (ADRs 0863, 0864): an `Option` of a `RefCell<HashSet<&Fiber>>`, whose
// `Fiber` is equal only to itself, is a JS `Set` keyed by each fiber, none
// of whose borrows is checked, as none can meet another.

use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

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

struct Transition {
    updated_fibers: Option<RefCell<HashSet<&'static Fiber>>>,
}

fn update(transition: &Transition, fiber: &'static Fiber) {
    if let Some(updated) = &transition.updated_fibers {
        updated.borrow_mut().insert(fiber);
    }
}

fn finish(transition: &Transition) -> usize {
    match &transition.updated_fibers {
        Some(updated) => {
            let count = updated.borrow().len();
            updated.borrow_mut().clear();
            count
        }
        None => 0,
    }
}

fn main() {
    let a: &'static Fiber = Box::leak(Box::new(Fiber { lanes: 1 }));
    let b: &'static Fiber = Box::leak(Box::new(Fiber { lanes: 1 }));
    let transition = Transition {
        updated_fibers: Some(RefCell::new(HashSet::new())),
    };
    update(&transition, a);
    update(&transition, a);
    update(&transition, b);
    println!("{} {}", finish(&transition), finish(&transition));
    let none = Transition { updated_fibers: None };
    update(&none, a);
    println!("{} {}", finish(&none), a.lanes + b.lanes);
}
