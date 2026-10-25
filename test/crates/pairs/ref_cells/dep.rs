//! A library's `RefCell`s, as React's `Transition` holds its types (ADR
//! 0863): a field's `RefCell<Option<_>>` the value it holds, an `Rc` of a
//! `RefCell<Vec<String>>` the array, and an `Option` of a
//! `RefCell<HashSet<u32>>` the `Set`, none of whose borrows is checked, as
//! none can meet another.
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub struct Transition {
    pub types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,
    pub seen: Option<RefCell<HashSet<u32>>>,
}

pub fn start(prev: Option<&Transition>) -> Transition {
    let mut current = Transition {
        types: RefCell::new(None),
        seen: Some(RefCell::new(HashSet::new())),
    };
    *current.types.get_mut() = match prev {
        Some(prev) => prev.types.borrow().clone(),
        None => None,
    };
    current
}

pub fn add_type(transition: &Transition, ty: &str) {
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

pub fn types(transition: &Transition) -> Vec<String> {
    let types = transition.types.borrow().clone();
    types.map(|types| types.borrow().clone()).unwrap_or_default()
}

pub fn share_types(a: &Transition, b: &Transition) -> bool {
    let a = a.types.borrow().clone();
    let b = b.types.borrow().clone();
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
        _ => false,
    }
}

pub fn visit(transition: &Transition, id: u32) {
    if let Some(seen) = &transition.seen {
        seen.borrow_mut().insert(id);
    }
}

pub fn finish(transition: &Transition) -> usize {
    match &transition.seen {
        Some(seen) => {
            let count = seen.borrow().len();
            seen.borrow_mut().clear();
            count
        }
        None => 0,
    }
}
