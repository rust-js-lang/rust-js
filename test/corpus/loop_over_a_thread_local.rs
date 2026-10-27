// A loop over a thread-local, `while let Some(parent) = PARENT.with_borrow(
// Clone::clone)`, tests the module's variable itself, React's
// popToNextHostParent's `while (hydrationParentFiber != null) { switch
// (hydrationParentFiber.tag) { .. } }`: the copy each turn makes is the
// variable, read in place, where the loop sets it only after its last
// read (ADR 0277, amended; ADR 0860, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    tag: u8,
    parent: Option<Rc<Fiber>>,
}

thread_local! {
    static HYDRATION_PARENT_FIBER: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
    static ROOT_CONTEXT: Cell<bool> = const { Cell::new(false) };
}

fn pop_to_next_host_parent(fiber: &Fiber) -> u8 {
    HYDRATION_PARENT_FIBER.set(fiber.parent.clone());
    while let Some(hydration_parent_fiber) = HYDRATION_PARENT_FIBER.with_borrow(Clone::clone) {
        match hydration_parent_fiber.tag {
            5 | 13 => {
                ROOT_CONTEXT.set(false);
                return 5;
            }
            3 => {
                ROOT_CONTEXT.set(true);
                return 3;
            }
            _ => HYDRATION_PARENT_FIBER.set(hydration_parent_fiber.parent.clone()),
        }
    }
    0
}

fn first_tagged(fiber: &Fiber, tag: u8) -> u8 {
    HYDRATION_PARENT_FIBER.set(fiber.parent.clone());
    let mut seen = 0;
    while let Some(parent) = HYDRATION_PARENT_FIBER.with_borrow(Clone::clone) {
        seen += 1;
        if parent.tag == tag {
            break;
        }
        HYDRATION_PARENT_FIBER.set(parent.parent.clone());
    }
    seen
}

fn walk_to(fiber: &Fiber, tag: u8) {
    HYDRATION_PARENT_FIBER.set(fiber.parent.clone());
    while let Some(parent) = HYDRATION_PARENT_FIBER.with_borrow(Clone::clone) {
        if parent.tag == tag {
            ROOT_CONTEXT.set(true);
            break;
        }
        HYDRATION_PARENT_FIBER.set(parent.parent.clone());
    }
}

fn main() {
    let root = Rc::new(Fiber { tag: 3, parent: None });
    let host = Rc::new(Fiber { tag: 5, parent: Some(Rc::clone(&root)) });
    let leaf = Fiber { tag: 0, parent: Some(Rc::new(Fiber { tag: 7, parent: Some(Rc::clone(&root)) })) };
    let orphan = Fiber { tag: 0, parent: None };
    let in_host = Fiber { tag: 0, parent: Some(host) };
    println!(
        "{} {} {} {}",
        pop_to_next_host_parent(&leaf),
        ROOT_CONTEXT.get(),
        pop_to_next_host_parent(&in_host),
        ROOT_CONTEXT.get()
    );
    println!("{} {}", pop_to_next_host_parent(&orphan), HYDRATION_PARENT_FIBER.with_borrow(Option::is_none));
    println!("{} {}", first_tagged(&leaf, 3), first_tagged(&leaf, 9));
    ROOT_CONTEXT.set(false);
    walk_to(&leaf, 7);
    let tag = HYDRATION_PARENT_FIBER.with_borrow(|parent| parent.as_ref().map(|parent| parent.tag));
    println!("{} {}", ROOT_CONTEXT.get(), tag.unwrap_or(0));
}
