// An `else` of only comments after an `else if`, in a loop that goes on
// past it, keeps the `else if` as written and its `else` of only comments,
// as React's scheduleContextWorkOnParentPath has them: not the `else if`'s
// `if` inside an `else` under the comments (rd722).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    pub child_lanes: Cell<u32>,
    pub alternate: RefCell<Option<Rc<Fiber>>>,
    pub parent: RefCell<Option<Rc<Fiber>>>,
}

fn is_subset_of_lanes(set: u32, subset: u32) -> bool {
    set & subset == subset
}

fn merge_lanes(a: u32, b: u32) -> u32 {
    a | b
}

pub fn schedule(parent: Option<Rc<Fiber>>, render_lanes: u32, propagation_root: &Fiber) {
    let mut node = parent;
    while let Some(fiber) = &node {
        let alternate = fiber.alternate.borrow().clone();
        if !is_subset_of_lanes(fiber.child_lanes.get(), render_lanes) {
            fiber
                .child_lanes
                .set(merge_lanes(fiber.child_lanes.get(), render_lanes));
            if let Some(alternate) = &alternate {
                alternate
                    .child_lanes
                    .set(merge_lanes(alternate.child_lanes.get(), render_lanes));
            }
        } else if let Some(alternate) = &alternate
            && !is_subset_of_lanes(alternate.child_lanes.get(), render_lanes)
        {
            alternate
                .child_lanes
                .set(merge_lanes(alternate.child_lanes.get(), render_lanes));
        } else {
            // Neither alternate was updated.
            // This is why we continue the loop.
        }
        if std::ptr::eq(&**fiber, propagation_root) {
            break;
        }
        let next = fiber.parent.borrow().clone();
        node = next;
    }
}

fn fiber(lanes: u32, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        child_lanes: Cell::new(lanes),
        alternate: RefCell::new(None),
        parent: RefCell::new(parent),
    })
}

fn main() {
    let root = fiber(0, None);
    let middle = fiber(2, Some(root.clone()));
    *middle.alternate.borrow_mut() = Some(fiber(0, None));
    let leaf = fiber(0, Some(middle.clone()));
    schedule(Some(leaf.clone()), 2, &root);
    let alternate = middle.alternate.borrow().clone().unwrap();
    println!(
        "{} {} {} {}",
        leaf.child_lanes.get(),
        middle.child_lanes.get(),
        alternate.child_lanes.get(),
        root.child_lanes.get()
    );
    schedule(Some(leaf), 1, &middle);
    println!("{} {}", middle.child_lanes.get(), alternate.child_lanes.get());
}
