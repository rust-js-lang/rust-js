// A thread-local's `set` given a field's borrow, React's
// popToNextHostParent's `hydrationParentFiber = fiber.return;`, borrows the
// thread-local's own cell while the field's is borrowed: one only its key
// reaches, which no `with` lends, is never the field's, so the field stays
// the value it holds, not a checked cell everywhere (ADR 0960, amended).
use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    tag: u32,
    parent: RefCell<Option<Rc<Fiber>>>,
}

thread_local! {
    static HYDRATION_PARENT_FIBER: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

fn pop_to_next_host_parent(fiber: &Fiber) {
    HYDRATION_PARENT_FIBER.set(fiber.parent.borrow().clone());
}

fn parent_tag(fiber: &Fiber) -> Option<u32> {
    fiber.parent.borrow().as_ref().map(|parent| parent.tag)
}

fn main() {
    let root = Rc::new(Fiber { tag: 3, parent: RefCell::new(None) });
    let leaf = Fiber { tag: 5, parent: RefCell::new(Some(Rc::clone(&root))) };
    pop_to_next_host_parent(&leaf);
    let held = HYDRATION_PARENT_FIBER.with_borrow(|parent| parent.as_ref().map(|parent| parent.tag));
    println!("{held:?} {:?} {:?}", parent_tag(&leaf), parent_tag(&root));
    pop_to_next_host_parent(&root);
    println!("{}", HYDRATION_PARENT_FIBER.with_borrow(Option::is_none));
}
