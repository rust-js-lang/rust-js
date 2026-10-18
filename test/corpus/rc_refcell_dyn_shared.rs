//@ library-refused: another crate may share
// An `Rc<RefCell<C>>` made an `Rc<RefCell<dyn T>>` where it's shared, a
// listener a bus and its owner both hold, beside a tree whose parent is a
// `Weak`, which counts `Rc`s (ADR 0320). Where no implementor of `T` drops
// anything, neither does a `dyn T`, so the listener's `Rc` isn't counted;
// where one does, the `Rc` made a counted one is counted too, sharing its
// counts, and the cell is given its `impl` in place (ADR 0397).
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

trait Sink {
    fn take(&mut self, n: u32);
    fn total(&self) -> u32;
}

struct Counter {
    sum: u32,
}

struct Loud {
    name: &'static str,
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

impl Sink for Counter {
    fn take(&mut self, n: u32) {
        self.sum += n;
    }

    fn total(&self) -> u32 {
        self.sum
    }
}

impl Sink for Loud {
    fn take(&mut self, n: u32) {
        println!("{} takes {n}", self.name);
    }

    fn total(&self) -> u32 {
        0
    }
}

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
}

fn main() {
    let logger = Rc::new(RefCell::new(Logger { seen: Vec::new() }));
    let listeners: Vec<Rc<RefCell<dyn Listener>>> = vec![logger.clone(), Rc::new(RefCell::new(Logger { seen: Vec::new() }))];
    for listener in &listeners {
        listener.borrow_mut().notify("start");
    }
    listeners[0].borrow_mut().notify("stop");
    println!("{} {}", logger.borrow().count(), listeners[1].borrow().count());

    let root = Rc::new(Node { name: "root".into(), parent: RefCell::new(Weak::new()) });
    let leaf = Rc::new(Node { name: "leaf".into(), parent: RefCell::new(Weak::new()) });
    *leaf.parent.borrow_mut() = Rc::downgrade(&root);
    println!("{} of {}", leaf.name, leaf.parent.borrow().upgrade().map(|p| p.name.clone()).unwrap_or_default());

    let counter = Rc::new(RefCell::new(Counter { sum: 0 }));
    {
        let sinks: Vec<Rc<RefCell<dyn Sink>>> = vec![counter.clone(), Rc::new(RefCell::new(Loud { name: "loud" }))];
        for (i, sink) in sinks.iter().enumerate() {
            sink.borrow_mut().take(i as u32 + 1);
        }
        println!("{} {} {}", sinks[0].borrow().total(), Rc::strong_count(&sinks[0]), Rc::strong_count(&counter));
    }
    println!("{} {}", counter.borrow().total(), Rc::strong_count(&counter));
}
