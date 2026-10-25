//! A library's type equal only to itself, as React's `Fiber` is (ADR 0864):
//! a `Transition` keeps the fibers it updated in a `Set`, each by itself.
use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

pub struct Fiber {
    pub tag: u8,
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

pub struct Transition {
    pub updated_fibers: Option<RefCell<HashSet<&'static Fiber>>>,
}

pub fn start() -> Transition {
    Transition {
        updated_fibers: Some(RefCell::new(HashSet::new())),
    }
}

pub fn updated(transition: &Transition) -> usize {
    transition
        .updated_fibers
        .as_ref()
        .map_or(0, |fibers| fibers.borrow().len())
}
