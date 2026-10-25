//! A library that counts the `Rc`s of one type and not another's (ADR
//! 0852): React's context, read by a generic function of any `T`, which
//! drops nothing of it, is the value itself; a generic `Slot<T>` its code
//! owns, whose last `Rc` may drop a consumer's value with a destructor, is
//! counted, and so is every `Rc<Slot<_>>`; a shelf of `Rc<Item<T>>`s it
//! only reads through a reference, which drops none, is the value itself.
use std::cell::Cell;
use std::rc::{Rc, Weak};

pub struct Context<T: 'static> {
    pub value: Cell<T>,
    pub provider: Cell<Weak<Context<T>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub consumer: Cell<Option<Rc<Consumer<T>>>>,
}

pub struct Consumer<T: 'static> {
    pub context: Weak<Context<T>>,
}

pub fn create_context<T: Copy>(default_value: T) -> Rc<Context<T>> {
    let context = Rc::new(Context {
        value: Cell::new(default_value),
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

/// Of any `T`, as React's `useContext`: it reads the context, and owns
/// only its consumer for a while, which drops nothing of a `T`.
pub fn use_context<T>(context: &Context<T>) -> bool {
    let consumer = context.consumer.take();
    let found = consumer.is_some();
    context.consumer.set(consumer);
    found
}

pub struct Slot<T> {
    pub value: T,
}

/// Two owners of a value of any `T`, which the last of them drops.
pub fn share<T>(value: T) -> (Rc<Slot<T>>, Rc<Slot<T>>) {
    let slot = Rc::new(Slot { value });
    (Rc::clone(&slot), slot)
}

pub struct Item<T> {
    pub value: T,
}

/// `Rc`s of a value of any `T`, which only a crate that owns a shelf drops.
pub struct Shelf<T> {
    pub items: Vec<Rc<Item<T>>>,
}

/// Of any `T`, reading a shelf it doesn't own, which drops nothing.
pub fn shelved<T>(shelf: &Shelf<T>) -> usize {
    shelf.items.len()
}
