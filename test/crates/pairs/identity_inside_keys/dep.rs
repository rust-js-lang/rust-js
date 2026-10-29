//! A library's type equal only to itself, as React's reconciler's `Fiber`
//! is (ADR 0864), and a key holding one, whose derived `Hash` and `Eq` find
//! the fiber by itself; and the library's own map of such keys, which its
//! consumer reads and adds to (ADR 1015, amended).
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub struct Fiber {
    pub tag: u32,
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

#[derive(PartialEq, Eq, Hash)]
pub struct Key {
    pub fiber: Rc<Fiber>,
    pub lane: u32,
}

pub fn key(fiber: &Rc<Fiber>, lane: u32) -> Key {
    Key {
        fiber: Rc::clone(fiber),
        lane,
    }
}

pub fn lanes(fibers: &[Rc<Fiber>]) -> HashMap<Key, usize> {
    let mut lanes = HashMap::new();
    for (i, fiber) in fibers.iter().enumerate() {
        lanes.insert(key(fiber, 1), i);
    }
    lanes
}
