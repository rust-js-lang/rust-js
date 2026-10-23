// A `Weak` shows none of what it points at, `(Weak)`, as std's does, in a
// struct's `Debug` too.
use std::cell::RefCell;
use std::rc::{Rc, Weak};
#[derive(Debug)]
struct Node {
    val: i32,
    parent: RefCell<Weak<Node>>,
    kids: Vec<Rc<Node>>,
}
fn main() {
    let root = Rc::new(Node {
        val: 1,
        parent: RefCell::new(Weak::new()),
        kids: vec![],
    });
    let kid = Rc::new(Node {
        val: 2,
        parent: RefCell::new(Rc::downgrade(&root)),
        kids: vec![],
    });
    println!("{:?}", kid);
    println!("{:#?}", kid);
    let w: Weak<i32> = Weak::new();
    println!("{:?} {:?}", w, Rc::downgrade(&Rc::new(5)).upgrade());
    let s = std::sync::Arc::new(3);
    println!("{:?}", std::sync::Arc::downgrade(&s));
}
