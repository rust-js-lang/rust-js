//! A library that reads no count of its `Rc`s, as React's `createContext`
//! and `lazy` read none: it counts none, each the value itself, and hands
//! them to its consumers as they are (ADR 0844).
use std::cell::Cell;
use std::rc::{Rc, Weak};

pub struct Context {
    pub value: Cell<u32>,
    pub provider: Cell<Weak<Context>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub consumer: Cell<Option<Rc<Consumer>>>,
}

pub struct Consumer {
    pub context: Weak<Context>,
}

pub fn create_context(value: u32) -> Rc<Context> {
    let context = Rc::new(Context {
        value: Cell::new(value),
        // These are circular
        provider: Cell::new(Weak::new()),
        consumer: Cell::new(None),
    });
    context.provider.set(Rc::downgrade(&context));
    context.consumer.set(Some(Rc::new(Consumer {
        context: Rc::downgrade(&context),
    })));
    context
}

pub fn bump(context: &Rc<Context>) -> Rc<Context> {
    context.value.set(context.value.get() + 1);
    Rc::clone(context)
}

/// A shared cell of a type parameter's value that can't have a destructor,
/// as React's context's `T: Copy`: generic, and still the value itself.
pub fn shared_cell<T: Copy>(value: T) -> Rc<Cell<T>> {
    Rc::new(Cell::new(value))
}

/// Generic code given a consumer's value by a type parameter, which it
/// clones by what it's given, whatever that counts.
pub fn pair<T: Clone>(t: T) -> (T, T) {
    (t.clone(), t)
}
