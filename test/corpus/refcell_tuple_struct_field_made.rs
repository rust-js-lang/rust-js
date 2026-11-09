//@ run-pass
// A tuple struct's `RefCell` field is an item of the array it is, the cell
// by its type alone, where it's made and where it's read (#1082): of a
// generic one made of a `Vec` there, `[{ value }]`, read so where it's a
// `RefCell<Vec<i32>>`; and of one boxed as a `RefCell` of an `Rc`, made
// and read as that box. Not a property that holds the cell's value, as a
// struct's named field is (ADR 0863, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Cursor<T>(pub RefCell<T>);

pub fn cursor<T>(value: T) -> Cursor<T> {
    Cursor(RefCell::new(value))
}

pub struct Node(pub RefCell<Option<Rc<Node>>>, pub Cell<u32>);

pub struct Root(pub RefCell<Rc<Node>>, pub u32);

fn total(log: &RefCell<Vec<i32>>) -> i32 {
    log.borrow().iter().sum()
}

fn main() {
    let c = cursor(vec![1, 2]);
    c.0.borrow_mut().push(3);
    let log = RefCell::new(vec![4, 5]);
    log.borrow_mut().push(6);
    println!("{:?} {}", c.0.borrow(), total(&log));
    let replaced = c.0.replace(vec![8]);
    println!("{replaced:?} {:?}", c.0.borrow());

    let leaf = Rc::new(Node(RefCell::new(None), Cell::new(7)));
    let node = Rc::new(Node(RefCell::new(None), Cell::new(1)));
    *node.0.borrow_mut() = Some(leaf);
    let root = Root(RefCell::new(node), 2);
    let below = root.0.borrow().0.borrow().clone().unwrap();
    below.1.set(below.1.get() + root.1);
    println!("{} {}", below.1.get(), root.0.borrow().1.get());
}
