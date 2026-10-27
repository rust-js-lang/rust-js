//@ run-fail: RefCell already borrowed
// `contains_guarded_borrow.rs`'s merge into a handle the loop sets again
// after the test, which may then be the loop's cell: both borrows are
// checked, and it panics as Rust's does (ADR 0863, amended).
use std::cell::RefCell;
use std::rc::Rc;

fn merge(types: &Rc<RefCell<Vec<char>>>, other: &Rc<RefCell<Vec<char>>>) {
    let mut into = Rc::clone(other);
    for t in types.borrow().iter() {
        if !into.borrow().contains(t) {
            into = Rc::clone(types);
            into.borrow_mut().push(*t);
        }
    }
}

pub fn main() {
    let chars = Rc::new(RefCell::new(vec!['a', 'b']));
    println!("{:?}", chars.borrow());
    merge(&chars, &Rc::new(RefCell::new(vec!['b'])));
}
