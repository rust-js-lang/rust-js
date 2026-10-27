//@ run-fail: RefCell already borrowed
// `contains_guarded_borrow.rs`'s merge of floats: `NaN` isn't equal to
// itself, so where the two are one cell it isn't found, and the push's
// borrow meets the loop's. Both are checked, and it panics as Rust's does
// (ADR 0863, amended).
use std::cell::RefCell;
use std::rc::Rc;

fn merge(values: &Rc<RefCell<Vec<f64>>>, into: &Rc<RefCell<Vec<f64>>>) {
    for v in values.borrow().iter() {
        if !into.borrow().contains(v) {
            into.borrow_mut().push(*v);
        }
    }
}

pub fn main() {
    let f = Rc::new(RefCell::new(vec![1.5, f64::NAN]));
    let g = Rc::new(RefCell::new(Vec::new()));
    merge(&f, &g);
    println!("{}", g.borrow().len());
    merge(&f, &f);
}
