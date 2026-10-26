//! A library's generic functions as React's `createContext` and
//! `useState` are, given what its own crate's calls give them, nothing: its
//! JS is the program's, which JS calls as React's own JS does. A consumer
//! that gives them a value to copy or drop compiles its own (ADR 0914).
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

/// The dispatcher a hook is called in, as React's `resolveDispatcher`: none
/// outside of a render, which panics.
fn resolve_dispatcher(dispatcher: Option<&'static str>) -> &'static str {
    match dispatcher {
        Some(dispatcher) => dispatcher,
        None => panic!("Invalid hook call."),
    }
}

pub fn use_state<S>(initial_state: S, dispatcher: Option<&'static str>) -> (S, &'static str) {
    let dispatcher = resolve_dispatcher(dispatcher);
    (initial_state, dispatcher)
}

/// Asks its dispatcher before it drops what it's given: a panic drops it
/// then, by the flag optimized MIR keeps of whether it's still there.
pub fn dispatched<T>(value: T, dispatcher: Option<&'static str>) -> usize {
    let length = resolve_dispatcher(dispatcher).len();
    drop(value);
    length
}

/// Two copies of `value`, its own copy here, and `dropped` dropped.
pub fn pair_of<T: Copy, D>(value: T, dropped: D) -> (T, T) {
    drop(dropped);
    (value, value)
}

/// Two of what it's given, which its own crate gives a struct it changes:
/// it takes a copy, as its contract says, and its consumer gives it one.
pub fn twice<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

#[derive(Clone, Copy)]
struct Mark {
    n: i32,
}

pub fn marks() -> i32 {
    let (mut a, b) = twice(Mark { n: 1 });
    a.n += 10;
    a.n + b.n
}
