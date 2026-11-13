//@ run-fail: RefCell already mutably borrowed
use std::cell::RefCell;

fn length_by(f: impl Fn() -> usize) -> i32 {
    f() as i32
}

fn main() {
    let log = RefCell::new(vec![1]);
    log.borrow_mut().push(length_by(|| log.borrow().len()));
    println!("{:?}", log.borrow());
}
