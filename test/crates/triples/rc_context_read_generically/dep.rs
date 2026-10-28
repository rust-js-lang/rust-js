// React's context, `react`'s: made of a `T: Copy`, and pointing at itself
// by a `Weak`, so its `Rc`s count nothing here (ADR 0852).
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub struct Context<T: 'static> {
    pub current_value: RefCell<T>,
    pub provider: Cell<Weak<Context<T>>>,
}

pub fn create_context<T: Copy>(default_value: T) -> Rc<Context<T>> {
    let context = Rc::new(Context {
        current_value: RefCell::new(default_value),
        provider: Cell::new(Weak::new()),
    });
    context.provider.set(Rc::downgrade(&context));
    context
}
