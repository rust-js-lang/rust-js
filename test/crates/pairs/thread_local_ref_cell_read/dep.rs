// React's reconciler's `ReactCurrentFiber`: a thread-local `RefCell` its
// module sets and reads, `export let current`, which React DOM reads as
// `const child = current;` (gap rd76).
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

pub fn current_tag() -> Option<u32> {
    CURRENT.with_borrow(|current| current.as_ref().map(|fiber| fiber.tag))
}
