// A `while let` that ends its function is its test and its `break`s where
// the program wrote them, `while (parent != null) { if (..) { ..; break; }
// parent = parent.return; }`, as one followed by code is: MIR's way out of
// it is the function's `return;`, which is the loop's `break` (ADR 0702,
// amended), as React's commitDeletionEffects finds a host parent.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Fiber {
    tag: Cell<u8>,
    state: Cell<u32>,
    parent: RefCell<Option<Rc<Fiber>>>,
}

thread_local! {
    static HOST_PARENT: Cell<u32> = const { Cell::new(0) };
}

fn find_host_parent(fiber: &Rc<Fiber>) {
    let mut parent = Some(Rc::clone(fiber));
    while let Some(current) = parent {
        if current.tag.get() == 2 {
            HOST_PARENT.set(current.state.get());
            break;
        }
        parent = current.parent.borrow().clone();
    }
}

fn depth(fiber: &Rc<Fiber>, log: &mut Vec<u32>) {
    let mut parent = Some(Rc::clone(fiber));
    while let Some(current) = parent {
        log.push(current.state.get());
        parent = current.parent.borrow().clone();
    }
}

fn chain(tags: &[u8]) -> Rc<Fiber> {
    let mut last: Option<Rc<Fiber>> = None;
    for (i, &tag) in tags.iter().enumerate().rev() {
        last = Some(Rc::new(Fiber {
            tag: Cell::new(tag),
            state: Cell::new(i as u32 * 10 + 1),
            parent: RefCell::new(last),
        }));
    }
    last.unwrap()
}

fn main() {
    find_host_parent(&chain(&[1, 1, 2, 2]));
    println!("{}", HOST_PARENT.get());
    find_host_parent(&chain(&[1, 3]));
    println!("{}", HOST_PARENT.get());
    let mut log = Vec::new();
    depth(&chain(&[1, 2, 3]), &mut log);
    println!("{log:?}");
}
