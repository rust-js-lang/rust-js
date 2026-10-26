//@ run-fail: RefCell already borrowed
// A generic body's moment's borrow of a `RefCell<Vec<T>>`, which may be
// any `RefCell<Vec<_>>`, meets the borrow the cell's own `clone` holds
// while what it holds is cloned: what `T` is isn't known there, so the
// cell may have a `Clone` of its own that runs what borrows it, and its
// borrow is checked (ADRs 0580, 0863).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    list: Weak<RefCell<Vec<Node>>>,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        if let Some(list) = self.list.upgrade() {
            flip(&list);
        }
        Node { list: Weak::new() }
    }
}

fn flip<T>(cell: &RefCell<Vec<T>>) {
    cell.borrow_mut().reverse();
}

fn main() {
    let list = Rc::new_cyclic(|me| RefCell::new(vec![Node { list: me.clone() }]));
    flip(&list);
    println!("{}", list.borrow().len());
    let copy = RefCell::clone(&list);
    println!("{}", copy.borrow().len());
}
