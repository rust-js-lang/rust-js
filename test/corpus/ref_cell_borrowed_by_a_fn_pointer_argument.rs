//@ run-fail: RefCell already mutably borrowed
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<i32>> = RefCell::new(vec![1]);
}

fn length() -> i32 {
    LOG.with_borrow(|log| log.len() as i32)
}

fn add(f: fn() -> i32) {
    LOG.with(|log| log.borrow_mut().push(f()));
}

fn main() {
    let mut picks: Vec<fn() -> i32> = vec![|| 0];
    picks.push(length);
    let pick = picks.pop().unwrap_or(|| 0);
    add(pick);
    LOG.with_borrow(|log| println!("{log:?}"));
}
