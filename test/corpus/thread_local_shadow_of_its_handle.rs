//@ library-differs
// A public thread-local's `with_borrow_mut` closure, applied where it's
// called, is given a handle on the thread-local's place; the closure's
// shadow of its parameter, `let current = current.get_or_insert_with(..)`,
// holds what that handle reads, the thread-local itself: `current ??=
// {..}; current.start.push(n)`, no holder of it, as React's work loop adds
// a transition's callbacks (ADR 1150).
use std::cell::RefCell;

pub struct Callbacks {
    pub start: Option<Vec<u32>>,
    pub done: Option<u32>,
}

thread_local! {
    pub static CURRENT_CALLBACKS: RefCell<Option<Callbacks>> = const { RefCell::new(None) };
}

pub fn add_start(n: u32) {
    CURRENT_CALLBACKS.with_borrow_mut(|current_callbacks| {
        let current_callbacks = current_callbacks.get_or_insert_with(|| Callbacks {
            start: Some(vec![]),
            done: None,
        });
        current_callbacks.start.get_or_insert_with(Vec::new).push(n);
    });
}

fn main() {
    add_start(1);
    add_start(2);
    CURRENT_CALLBACKS.with_borrow(|current| {
        let current = current.as_ref().expect("added");
        println!("{:?} {:?}", current.start, current.done);
    });
}
