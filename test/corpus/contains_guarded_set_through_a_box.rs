//@ run-fail: RefCell already borrowed
// `contains_guarded_set_again.rs`'s handle held in a box and set again
// through it, `*into = Rc::clone(types)`: set again as the variable is, so
// the loop's borrow and the `then`'s are both checked, and it panics as
// Rust's does (#1000).
use std::cell::RefCell;
use std::rc::Rc;

fn merge(types: &Rc<RefCell<Vec<char>>>, other: &Rc<RefCell<Vec<char>>>) {
    let mut into = Box::new(Rc::clone(other));
    for t in types.borrow().iter() {
        if !into.borrow().contains(t) {
            *into = Rc::clone(types);
            into.borrow_mut().push(*t);
        }
    }
}

pub fn main() {
    let chars = Rc::new(RefCell::new(vec!['a', 'b']));
    println!("{:?}", chars.borrow());
    merge(&chars, &Rc::new(RefCell::new(vec!['b'])));
}
