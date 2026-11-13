//@ run-fail: RefCell already mutably borrowed
use std::cell::RefCell;

fn add(log: &RefCell<Vec<i32>>, f: &dyn Fn() -> i32) {
    log.borrow_mut().push(f());
}

fn main() {
    let log = RefCell::new(vec![1]);
    add(&log, &|| log.borrow().len() as i32);
    println!("{:?}", log.borrow());
}
