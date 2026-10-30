//@ library-differs
// Listeners, `Rc<RefCell<dyn Listener>>`, shared with their owner, beside a
// tree whose parent is a `Weak`, which counts its `Rc`s (ADR 0320). No
// implementor of `Listener` drops anything, so neither does a `dyn
// Listener`, and its `Rc` is the value it points at, as its owner's is.
use std::cell::RefCell;
use std::rc::{Rc, Weak};

trait Listener {
    fn notify(&mut self, event: &str);
    fn count(&self) -> usize;
}

struct Logger {
    seen: Vec<String>,
}

impl Listener for Logger {
    fn notify(&mut self, event: &str) {
        self.seen.push(event.to_uppercase());
    }

    fn count(&self) -> usize {
        self.seen.len()
    }
}

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
}

fn main() {
    let logger = Rc::new(RefCell::new(Logger { seen: Vec::new() }));
    let listeners: Vec<Rc<RefCell<dyn Listener>>> = vec![logger.clone()];
    listeners[0].borrow_mut().notify("a");
    let root = Rc::new(Node { name: "root".into(), parent: RefCell::new(Weak::new()) });
    let leaf = Rc::new(Node { name: "leaf".into(), parent: RefCell::new(Weak::new()) });
    *leaf.parent.borrow_mut() = Rc::downgrade(&root);
    let parent = leaf.parent.borrow().upgrade().map(|p| p.name.clone()).unwrap_or_default();
    println!("{} {} {}", logger.borrow().count(), leaf.name, parent);
}
