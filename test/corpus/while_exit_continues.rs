// React's `findFirstSuspended`: a `while` whose test, failing, runs what
// goes on with the loop, `node = return_fiber`, before the loop's test
// again. That test's exit holds a `continue` of the inner loop, so it
// stays in it, not after it, where the `continue` would go on with the
// outer one (ADR 0702, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Fiber {
    name: &'static str,
    tag: Cell<u32>,
    r#return: RefCell<Option<Rc<Fiber>>>,
    sibling: RefCell<Option<Rc<Fiber>>>,
    child: RefCell<Option<Rc<Fiber>>>,
}

fn fiber(name: &'static str, tag: u32) -> Rc<Fiber> {
    Rc::new(Fiber {
        name,
        tag: Cell::new(tag),
        r#return: RefCell::new(None),
        sibling: RefCell::new(None),
        child: RefCell::new(None),
    })
}

fn find_first_suspended(row: &Rc<Fiber>) -> Option<Rc<Fiber>> {
    let mut node = Rc::clone(row);
    loop {
        if node.tag.get() == 13 {
            return Some(node);
        }
        let child = node.child.borrow().clone();
        if let Some(child) = child {
            child.r#return.replace(Some(Rc::clone(&node)));
            node = child;
            continue;
        }
        if Rc::ptr_eq(&node, row) {
            return None;
        }
        while node.sibling.borrow().is_none() {
            let return_fiber = node.r#return.borrow().clone();
            match return_fiber {
                Some(return_fiber) if !Rc::ptr_eq(&return_fiber, row) => node = return_fiber,
                _ => return None,
            }
        }
        let sibling = node.sibling.borrow().clone().unwrap();
        let return_fiber = node.r#return.borrow().clone();
        sibling.r#return.replace(return_fiber);
        node = sibling;
    }
}

fn main() {
    // row -> a -> b, the deepest, with no sibling; a's sibling c is
    // suspended. Past b, the loop goes up to a and on to its sibling c, not
    // back to a's own child, b, again.
    let row = fiber("row", 0);
    let a = fiber("a", 5);
    let b = fiber("b", 0);
    let c = fiber("c", 13);
    row.child.replace(Some(a.clone()));
    a.child.replace(Some(b.clone()));
    a.sibling.replace(Some(c.clone()));
    println!("{:?}", find_first_suspended(&row).map(|f| f.name));
    // None suspended: up to the row, `None`.
    c.tag.set(0);
    println!("{:?}", find_first_suspended(&row).map(|f| f.name));
    println!("{:?}", find_first_suspended(&b).map(|f| f.name));
}
