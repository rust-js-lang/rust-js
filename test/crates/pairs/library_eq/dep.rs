//! A library's types compared from its consumer: one equal only to itself
//! by its `PartialEq` and `Hash`, as React's reconciler's `Fiber` is; one
//! whose `PartialEq` alone is its address's, as its `Update` is; one whose
//! `PartialEq` is derived; one whose `eq` is its own; and generic
//! functions that compare what they're given.
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub struct Fiber {
    pub tag: u32,
    pub parent: RefCell<Option<Rc<Fiber>>>,
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

pub fn make(tag: u32) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag,
        parent: RefCell::new(None),
    })
}

pub struct Update {
    pub lane: u32,
    pub next: RefCell<Option<Rc<Update>>>,
}

impl PartialEq for Update {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub fn update(lane: u32) -> Rc<Update> {
    Rc::new(Update {
        lane,
        next: RefCell::new(None),
    })
}

#[derive(PartialEq, Debug)]
pub struct Point {
    pub x: i32,
    pub label: String,
}

/// A derived `PartialEq` of a part equal only to itself.
#[derive(PartialEq)]
pub struct Dispatch {
    pub instance: Option<Rc<Fiber>>,
    pub name: String,
}

pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

pub fn same<T: PartialEq>(a: &T, b: &T) -> bool {
    a == b
}

pub fn first_index<T: PartialEq>(items: &[T], x: &T) -> Option<usize> {
    items.iter().position(|item| item == x)
}
