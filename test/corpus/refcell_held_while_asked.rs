//@ library-refused: another crate may share
// A borrow is held while code that can ask whether its cell is borrowed
// runs, as std holds it (ADR 0328): a temporary's `Drop`, an item's
// `Clone` a std method calls, a closure, and a cell's own `clone`, `==` and
// `{:?}` of what it holds.

use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

thread_local! {
    static SHARED: RefCell<Vec<i32>> = RefCell::new(vec![1]);
}

fn asks_shared() -> bool {
    SHARED.with(|s| s.try_borrow_mut().is_err())
}

/// Asks, as it's dropped, whether its cell is borrowed.
struct Probe<'a>(&'a RefCell<Vec<i32>>, usize);

impl Drop for Probe<'_> {
    fn drop(&mut self) {
        println!("drop sees it borrowed: {}", self.0.try_borrow_mut().is_err());
    }
}

/// Asks, as it's cloned, whether its cell is borrowed.
struct Peek<'a>(&'a RefCell<Vec<i32>>);

impl Clone for Peek<'_> {
    fn clone(&self) -> Self {
        println!("clone sees it borrowed: {}", self.0.try_borrow_mut().is_err());
        Peek(self.0)
    }
}

/// Held in the cell it asks about, through a `Weak`.
struct Node {
    me: Weak<RefCell<Node>>,
}

impl Node {
    fn asks(&self) -> bool {
        match self.me.upgrade() {
            Some(me) => me.try_borrow_mut().is_err(),
            None => false,
        }
    }
}

impl Clone for Node {
    fn clone(&self) -> Self {
        println!("node clone sees it borrowed: {}", self.asks());
        Node { me: Weak::new() }
    }
}

impl PartialEq for Node {
    fn eq(&self, _: &Node) -> bool {
        println!("node == sees it borrowed: {}", self.asks());
        true
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({})", self.asks())
    }
}

fn main() {
    let cell = RefCell::new(vec![1, 2]);
    println!("{} {}", cell.borrow().len(), cell.try_borrow_mut().is_err());
    let n = cell.borrow().len() + Probe(&cell, 0).1;
    println!("{n}");
    let peeks = vec![Peek(&cell)];
    let m = cell.borrow().len() + peeks.clone().len();
    println!("{m}");
    let options = vec![Some(Peek(&cell))];
    let k = cell.borrow().len() + options.clone().len();
    println!("{k}");
    let asked: Vec<bool> = cell.borrow().iter().map(|_| cell.try_borrow_mut().is_err()).collect();
    println!("{asked:?}");
    let seen: Vec<bool> = SHARED.with(|s| s.borrow().iter().map(|_| asks_shared()).collect());
    println!("{seen:?}");

    let node = Rc::new_cyclic(|me| RefCell::new(Node { me: me.clone() }));
    let copy = RefCell::clone(&node);
    println!("{} {:?}", *node == copy, node);
    println!("{}", node.borrow().asks());
}
