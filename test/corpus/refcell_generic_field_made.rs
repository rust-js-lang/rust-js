//@ run-pass
// A generic struct's `RefCell<T>` field, made of its value where the
// struct is made, is that field's own, no `RefCell<T>` of any value that
// generic code is given (ADR 0863, amended): a `RefCell<Vec<i32>>`
// elsewhere, lent and borrowed, is the array, and a thread-local's cell is
// the thread-local's own, its module's variable.
use std::cell::RefCell;

pub struct Cursor<T> {
    pub current: RefCell<T>,
}

pub fn cursor<T>(value: T) -> Cursor<T> {
    Cursor {
        current: RefCell::new(value),
    }
}

fn total(log: &RefCell<Vec<i32>>) -> i32 {
    log.borrow().iter().sum()
}

thread_local! {
    static STACK: RefCell<Vec<i32>> = RefCell::new(vec![1]);
}

fn main() {
    let c = cursor(vec![1, 2]);
    c.current.borrow_mut().push(3);
    let log = RefCell::new(vec![4, 5]);
    log.borrow_mut().push(6);
    STACK.with_borrow_mut(|stack| stack.push(7));
    println!(
        "{:?} {} {:?}",
        c.current.borrow(),
        total(&log),
        STACK.with_borrow(|stack| stack.clone())
    );
    let replaced = c.current.replace(vec![8]);
    println!("{replaced:?} {:?}", c.current.borrow());
}
