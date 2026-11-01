//@ run-fail: RefCell already mutably borrowed
// A cell kept in a thread-local is each one taken from it (ADR 0863,
// amended): a borrow held while a closure takes the same cell from the
// thread-local and borrows it is checked, and the second panics, as std's
// does. Another of its type, never kept there, is never checked.

use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CACHE: RefCell<Vec<Rc<RefCell<Vec<i32>>>>> = const { RefCell::new(Vec::new()) };
}

fn remember(cell: Rc<RefCell<Vec<i32>>>) {
    CACHE.with_borrow_mut(|cache| cache.push(cell));
}

fn first() -> Rc<RefCell<Vec<i32>>> {
    CACHE.with_borrow(|cache| Rc::clone(&cache[0]))
}

fn main() {
    remember(Rc::new(RefCell::new(vec![1])));
    let other = Rc::new(RefCell::new(vec![5]));
    other.borrow_mut().push(6);
    println!("{}", other.borrow().len());
    let cell = first();
    let again = || first().borrow().len();
    let mut held = cell.borrow_mut();
    held.push(2);
    println!("{}", again());
    println!("{}", held.len());
}
