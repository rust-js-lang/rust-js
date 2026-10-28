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

struct Restore(Option<Rc<Fiber>>);

impl Drop for Restore {
    fn drop(&mut self) {
        CURRENT.set(self.0.take());
    }
}

/// React's own five-argument form, which reads what it restores by a
/// closure it gives std's `with_borrow`: a closure of a generic function
/// is of its type parameters, but holds none of the `()`s it's given
/// (gap rd85).
pub fn run_with_fiber_in_dev<A0, A1, A2, A3, A4, T>(
    fiber: Option<Rc<Fiber>>,
    callback: impl FnOnce(A0, A1, A2, A3, A4) -> T,
    arg0: A0,
    arg1: A1,
    arg2: A2,
    arg3: A3,
    arg4: A4,
) -> T {
    let previous = CURRENT.with_borrow(|current| current.clone());
    CURRENT.set(fiber.clone());
    let _restore = Restore(previous);
    callback(arg0, arg1, arg2, arg3, arg4)
}

pub fn own_in_dev() -> u32 {
    run_with_fiber_in_dev(None, |a, b, c, d, e| a + b + c + d + e, 1, 2, 3, 4, 5)
}
