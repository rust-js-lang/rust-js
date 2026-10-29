//! React's reconciler's `runWithFiberInDEV`, its fiber set by a `pub fn`
//! of its own, which a consumer's copy calls: given a value with a
//! destructor, the copy is compiled from its optimized MIR, whose cleanup
//! tests a drop flag of each argument it's given (#394).
use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u32,
}

thread_local! {
    pub static CURRENT: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

pub fn set_current(fiber: Option<Rc<Fiber>>) {
    CURRENT.set(fiber);
}

pub fn tag_now() -> Option<u32> {
    CURRENT.with_borrow(|current| current.as_ref().map(|fiber| fiber.tag))
}

struct Restore(Option<Rc<Fiber>>);

impl Drop for Restore {
    fn drop(&mut self) {
        set_current(self.0.take());
    }
}

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
    set_current(fiber.clone());
    let _restore = Restore(previous);
    callback(arg0, arg1, arg2, arg3, arg4)
}
