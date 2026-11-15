// A `RefCell` field's `Option` tested, then read again and unwrapped:
// `node.sibling.borrow().clone().unwrap()` after `is_some()` holds, or
// after a `while .. is_none()` ends, is the field as it is where nothing
// runs between but that field's own reads (#1933, step 1(b)). Where
// something runs between that empties it, a user's `Clone`, a call, the
// unwrap panics as Rust's does.
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

struct Fiber {
    id: u32,
    sibling: RefCell<Option<Rc<Fiber>>>,
    parent: RefCell<Option<Rc<Fiber>>>,
}

fn fiber(id: u32) -> Rc<Fiber> {
    Rc::new(Fiber {
        id,
        sibling: RefCell::new(None),
        parent: RefCell::new(None),
    })
}

// Up past each fiber with no sibling, then its sibling's.
fn next_sibling(fiber: &Rc<Fiber>) -> u32 {
    let mut node = Rc::clone(fiber);
    while node.sibling.borrow().is_none() {
        let parent = node.parent.borrow().clone();
        match parent {
            Some(parent) => node = parent,
            None => return 0,
        }
    }
    let sibling = node.sibling.borrow().clone().unwrap();
    sibling.id
}

fn tested(node: &Fiber) -> u32 {
    if node.sibling.borrow().is_some() {
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}

// A clone of the crate's that empties the field, between: of the fiber a
// thread-local holds.
thread_local! {
    static EMPTIED: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

struct Emptier(u32);

impl Clone for Emptier {
    fn clone(&self) -> Self {
        EMPTIED.with_borrow(|emptied| {
            if let Some(emptied) = emptied {
                emptied.sibling.replace(None);
            }
        });
        Emptier(self.0 + 1)
    }
}

fn cloned_between(node: &Fiber, emptier: &Emptier) -> u32 {
    if node.sibling.borrow().is_some() {
        let again = emptier.clone();
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id + again.0;
    }
    0
}

// A call between that empties it.
fn called_between(node: &Fiber, between: &dyn Fn()) -> u32 {
    if node.sibling.borrow().is_some() {
        between();
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.id;
    }
    0
}

fn main() {
    let root = fiber(1);
    let a = fiber(2);
    let b = fiber(3);
    a.parent.replace(Some(Rc::clone(&root)));
    root.sibling.replace(Some(Rc::clone(&b)));
    println!("next sibling {}", next_sibling(&a));
    println!("next sibling {}", next_sibling(&b));
    println!("tested {} {}", tested(&root), tested(&a));
    EMPTIED.set(Some(Rc::clone(&root)));
    let emptier = Emptier(1);
    let cloned = catch_unwind(AssertUnwindSafe(|| cloned_between(&root, &emptier)));
    println!("cloned between {:?}", cloned.is_err());
    EMPTIED.set(None);
    root.sibling.replace(Some(Rc::clone(&b)));
    let called = catch_unwind(AssertUnwindSafe(|| {
        called_between(&root, &|| {
            root.sibling.replace(None);
        })
    }));
    println!("called between {:?}", called.is_err());
    root.sibling.replace(Some(Rc::clone(&b)));
    println!("called between {}", called_between(&root, &|| {}));
}
