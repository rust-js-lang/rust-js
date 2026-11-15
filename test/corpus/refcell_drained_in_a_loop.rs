// A `for` loop that drains a `RefCell`'s `Vec`, `v.borrow_mut().drain(..j
// + 1)`, its end the length before the loop (#1885). The first time round
// shortens the `Vec`, and no variable is written, so the loop's bound by
// the cell's length doesn't hold the next time: the drain past its end is
// checked, as Rust's panics say. So of an `Rc`'s cell and a reversed loop.
// A loop whose body calls nothing that may change the cell keeps its
// bound: `v.borrow()[j]` of `j` in `0..end`.
use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check(what: &str, f: impl FnOnce() -> u32) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn drained(v: &RefCell<Vec<u32>>, k: usize) -> u32 {
    let mut total = 0;
    let end = v.borrow().len();
    for j in k..end {
        total += v.borrow_mut().drain(..j + 1).count() as u32;
    }
    total
}

fn drained_from_the_back(v: &Rc<RefCell<Vec<u32>>>) -> u32 {
    let mut total = 0;
    let end = v.borrow().len();
    for j in (0..end).rev() {
        total += v.borrow_mut().drain(..j + 1).count() as u32;
    }
    total
}

fn read(v: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let end = v.borrow().len();
    for j in 0..end {
        total += v.borrow()[j];
    }
    total
}

fn main() {
    check("drained from 0", || drained(&RefCell::new(vec![1, 2, 3, 4]), 0));
    check("drained from 3", || drained(&RefCell::new(vec![1, 2, 3, 4]), 3));
    check("drained from the back", || drained_from_the_back(&Rc::new(RefCell::new(vec![1, 2, 3, 4]))));
    check("read", || read(&RefCell::new(vec![1, 2, 3, 4])));
}
