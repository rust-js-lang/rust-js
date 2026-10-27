//@ run-fail: RefCell already borrowed
// `contains_guarded_borrow.rs`'s merge of a value that isn't the loop's
// item: nothing says it's there where the two are one cell, so the push's
// borrow may meet the loop's. Both are checked, and it panics as Rust's
// does (ADR 0863, amended).
use std::cell::RefCell;
use std::rc::Rc;

fn add(ids: &Rc<RefCell<Vec<u32>>>, into: &Rc<RefCell<Vec<u32>>>, other: u32) {
    for _ in ids.borrow().iter() {
        if !into.borrow().contains(&other) {
            into.borrow_mut().push(other);
        }
    }
}

pub fn main() {
    let ids = Rc::new(RefCell::new(vec![1, 2]));
    let more = Rc::new(RefCell::new(vec![11]));
    add(&ids, &more, 12);
    println!("{:?}", more.borrow());
    add(&ids, &ids, 20);
}
