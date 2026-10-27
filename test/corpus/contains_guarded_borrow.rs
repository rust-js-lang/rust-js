// React's scheduleGesture merges a transition's types into its gesture's,
// each pushed only where it isn't there yet (ADR 0863, amended). Were the
// two one cell, each type would be there, equal to itself, so nothing is
// pushed: the push's borrow never meets the loop's, and neither is
// checked. `contains_guarded_float.rs`, `contains_guarded_other.rs` and
// `contains_guarded_set_again.rs` are those where it would.
use std::cell::RefCell;
use std::rc::Rc;

fn merge(types: &Rc<RefCell<Vec<String>>>, into: &Rc<RefCell<Vec<String>>>) {
    for t in types.borrow().iter() {
        if !into.borrow().contains(t) {
            into.borrow_mut().push(t.clone());
        }
    }
}

pub fn main() {
    let a = Rc::new(RefCell::new(vec!["x".to_string(), "y".to_string()]));
    let b = Rc::new(RefCell::new(vec!["y".to_string()]));
    merge(&a, &b);
    merge(&a, &a);
    println!("{:?} {:?}", a.borrow(), b.borrow());
}
