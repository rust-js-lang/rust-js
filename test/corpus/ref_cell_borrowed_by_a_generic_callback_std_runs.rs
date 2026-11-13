//@ run-fail: RefCell already mutably borrowed
use std::cell::RefCell;

fn add(log: &RefCell<Vec<i32>>, f: impl Fn(i32) -> i32) {
    log.borrow_mut().push(Some(1).map(&f).unwrap_or(0));
}

fn main() {
    let log = RefCell::new(vec![1]);
    add(&log, |x| x + log.borrow().len() as i32);
    println!("{:?}", log.borrow());
}
