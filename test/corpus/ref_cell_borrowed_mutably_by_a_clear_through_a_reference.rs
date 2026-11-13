//@ run-fail: RefCell already borrowed
// A `RefCell` a destructor borrows mutably, of values a `clear()` through a
// reference drops while a shared borrow of it is held: Rust's borrow check
// panics, `already borrowed`, as the cell's borrows stay checked (#1661).
use std::cell::RefCell;
use std::rc::Rc;

struct Pusher(Rc<RefCell<Vec<i32>>>);

impl Drop for Pusher {
    fn drop(&mut self) {
        self.0.borrow_mut().push(1);
    }
}

fn clear_all(g: &RefCell<Vec<Pusher>>) {
    g.borrow_mut().clear()
}

fn main() {
    let log = Rc::new(RefCell::new(vec![0]));
    let pushers = RefCell::new(vec![Pusher(Rc::clone(&log))]);
    let seen = log.borrow();
    clear_all(&pushers);
    println!("{}", seen.len());
}
