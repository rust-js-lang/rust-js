//! A library's generic functions of a `T: Copy`, as React's `createContext`
//! is (ADR 0289, amended): its consumer gives a copy of what may change in
//! place, a `Point` it changes, and none of a number, its own copy; JS gives
//! none, as its declarations say.
use std::cell::Cell;
use std::rc::Rc;

pub struct Context<T: 'static> {
    pub current_value: Cell<T>,
    pub current_value2: Cell<T>,
}

pub fn create_context<T: Copy>(default_value: T) -> Rc<Context<T>> {
    Rc::new(Context {
        current_value: Cell::new(default_value),
        current_value2: Cell::new(default_value),
    })
}

pub fn twice<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

pub fn first<T: Copy>(items: &[T]) -> Option<T> {
    items.first().copied()
}
