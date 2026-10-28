//! React's reconciler's `runWithFiberInDEV`: a generic function its own
//! calls give a number, which may never look like `None`, so its export
//! takes each `Some` of its type parameters as the value; it makes none.
use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u32,
}

thread_local! {
    pub static CURRENT: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

pub fn run_with_fiber<A0, T>(fiber: Option<Rc<Fiber>>, callback: impl FnOnce(A0) -> T, arg0: A0) -> T {
    let previous = CURRENT.replace(fiber);
    let result = callback(arg0);
    CURRENT.set(previous);
    result
}

pub fn tag_now() -> Option<u32> {
    CURRENT.with_borrow(|current| current.as_ref().map(|fiber| fiber.tag))
}

pub fn own() -> u32 {
    run_with_fiber(None, |n: u32| n + 1, 1)
}

/// One that makes a `Some` of what it's given, whose box a `()` needs.
pub fn wrapped<T>(value: T) -> Option<T> {
    Some(value)
}

pub fn wrapped_own() -> Option<u32> {
    wrapped(3)
}
