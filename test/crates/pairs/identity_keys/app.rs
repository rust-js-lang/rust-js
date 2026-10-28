//! A crate keying a set by a library's type equal only to itself, through
//! the library's struct that holds one, and comparing two by themselves.
use std::collections::HashSet;

use dep::{Fiber, Transition};

fn schedule(transition: &Transition, fiber: &'static Fiber) {
    if let Some(fibers) = &transition.updated_fibers {
        fibers.borrow_mut().insert(fiber);
    }
}

fn scheduled(transition: &Transition, fiber: &Fiber) -> bool {
    transition
        .updated_fibers
        .as_ref()
        .is_some_and(|fibers| fibers.borrow().contains(fiber))
}

pub fn main() {
    let a: &'static Fiber = Box::leak(Box::new(Fiber { tag: 1 }));
    let b: &'static Fiber = Box::leak(Box::new(Fiber { tag: 1 }));
    let transition = dep::start();
    schedule(&transition, a);
    schedule(&transition, a);
    println!("{} {}", scheduled(&transition, a), scheduled(&transition, b));
    schedule(&transition, b);
    println!("{} {} {}", dep::updated(&transition), a == b, a == a);
    let mut own: HashSet<&Fiber> = HashSet::new();
    own.insert(b);
    println!("{} {} {}", own.contains(a), own.contains(b), a.tag + b.tag);
}
