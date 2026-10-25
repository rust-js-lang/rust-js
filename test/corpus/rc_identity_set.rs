// React's root keeps the fibers that scheduled updates, each by itself
// (ADR 0864, amended): a `HashSet<Rc<Fiber>>`, whose `Fiber` is equal only
// to itself, is a JS `Set` keyed by each fiber, as an `Rc`'s `==` and
// `Hash` are what it points at's, its address's.

use std::cell::RefCell;
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

struct FiberRoot {
    memoized_updaters: RefCell<HashSet<Rc<Fiber>>>,
}

fn add_updater(root: &FiberRoot, fiber: &Rc<Fiber>) {
    root.memoized_updaters.borrow_mut().insert(Rc::clone(fiber));
}

fn main() {
    let a = Rc::new(Fiber { lanes: 1 });
    let b = Rc::new(Fiber { lanes: 1 });
    let root = FiberRoot {
        memoized_updaters: RefCell::new(HashSet::new()),
    };
    add_updater(&root, &a);
    add_updater(&root, &a);
    add_updater(&root, &b);
    let again = Rc::clone(&a);
    println!(
        "{} {} {} {}",
        root.memoized_updaters.borrow().len(),
        root.memoized_updaters.borrow().contains(&again),
        a == again,
        a == b
    );
    root.memoized_updaters.borrow_mut().remove(&b);
    println!("{} {}", root.memoized_updaters.borrow().len(), a.lanes + b.lanes);
}
