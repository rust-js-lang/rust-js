// A `RefCell` its destructors borrow, of values a thread-local's
// `with_borrow_mut(|v| v.clear())` drops: the closure's std call may drop a
// value of its type arguments, so `try_borrow_mut` is `false` while `seen`
// holds the cell (#1661).
use std::cell::RefCell;
use std::rc::Rc;

struct Guard(Rc<RefCell<Vec<i32>>>);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("free: {}", self.0.try_borrow_mut().is_ok());
    }
}

thread_local! {
    static GUARDS: RefCell<Vec<Guard>> = const { RefCell::new(Vec::new()) };
}

fn main() {
    let log = Rc::new(RefCell::new(vec![0]));
    GUARDS.with_borrow_mut(|v| v.push(Guard(Rc::clone(&log))));
    let n = {
        let seen = log.borrow();
        GUARDS.with_borrow_mut(|v| v.clear());
        seen.len()
    };
    println!("{n}");
}
