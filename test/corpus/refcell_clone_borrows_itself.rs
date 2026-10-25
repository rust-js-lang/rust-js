//@ run-fail: RefCell already borrowed
// A `RefCell`'s `clone` holds a borrow of it while what it holds is
// cloned: a `Clone` that borrows the same cell mutably finds it borrowed,
// so the cell's borrows are counted, though each the program writes is
// held for a moment (ADRs 0328, 0863).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    me: Weak<RefCell<Node>>,
    visits: u32,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        if let Some(me) = self.me.upgrade() {
            me.borrow_mut().visits += 1;
        }
        Node {
            me: Weak::new(),
            visits: self.visits,
        }
    }
}

fn main() {
    let node = Rc::new_cyclic(|me| RefCell::new(Node { me: me.clone(), visits: 0 }));
    node.borrow_mut().visits += 1;
    println!("{}", node.borrow().visits);
    let copy = RefCell::clone(&node);
    println!("{}", copy.borrow().visits);
}
