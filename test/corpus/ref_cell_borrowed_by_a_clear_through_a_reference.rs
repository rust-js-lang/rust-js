// A `RefCell` its destructors borrow, of values a `clear()` through a
// reference drops, `g.borrow_mut().clear()`: a std call may drop a value of
// each of its type arguments, so the cell's borrows stay checked, and
// `try_borrow_mut` is `false` while `seen` holds it (#1661).
use std::cell::RefCell;
use std::rc::Rc;

struct Guard(Rc<RefCell<Vec<i32>>>);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("free: {}", self.0.try_borrow_mut().is_ok());
    }
}

fn clear_all(g: &RefCell<Vec<Guard>>) {
    g.borrow_mut().clear()
}

fn main() {
    let log = Rc::new(RefCell::new(vec![0]));
    let guards = RefCell::new(vec![Guard(Rc::clone(&log))]);
    let n = {
        let seen = log.borrow();
        clear_all(&guards);
        seen.len()
    };
    println!("{n}");
}
