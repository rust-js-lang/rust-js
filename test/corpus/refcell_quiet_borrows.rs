// A `RefCell` none of whose borrows is held while code that may borrow it
// runs is never counted, so none of its borrows is checked: no borrow of it
// can meet another (ADR 0863). A field's, a local's, an `Rc`'s; borrowed in
// a `match` arm and replaced, iterated with a closure that borrows nothing,
// and an `Option` of one in a field.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

struct Transition {
    types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,
    seen: Option<RefCell<HashSet<u32>>>,
}

fn add_type(transition: &Transition, ty: &str) {
    let types = transition.types.borrow().clone();
    match types {
        None => {
            transition.types.replace(Some(Rc::new(RefCell::new(vec![ty.to_string()]))));
        }
        Some(types) => {
            if !types.borrow().iter().any(|t| t == ty) {
                types.borrow_mut().push(ty.to_string());
            }
        }
    }
}

fn nested(prev: Option<&Transition>) -> Transition {
    let current = Transition {
        types: RefCell::new(None),
        seen: Some(RefCell::new(HashSet::new())),
    };
    current.types.replace(match prev {
        Some(prev) => prev.types.borrow().clone(),
        None => None,
    });
    current
}

fn main() {
    let outer = nested(None);
    add_type(&outer, "a");
    add_type(&outer, "b");
    add_type(&outer, "a");
    let inner = nested(Some(&outer));
    add_type(&inner, "c");
    let shared = match (outer.types.borrow().clone(), inner.types.borrow().clone()) {
        (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
        _ => false,
    };
    println!("{:?} {shared}", outer.types.borrow().as_ref().map(|t| t.borrow().clone()));

    if let Some(seen) = &inner.seen {
        seen.borrow_mut().insert(3);
        seen.borrow_mut().insert(3);
        let count = seen.borrow().len();
        seen.borrow_mut().clear();
        println!("{count} {}", seen.borrow().len());
    }

    let log = RefCell::new(vec![1]);
    let before = log.replace(vec![2, 3]);
    log.borrow_mut().push(4);
    let total: i32 = log.borrow().iter().sum();
    println!("{before:?} {total} {:?}", log.take());
}
