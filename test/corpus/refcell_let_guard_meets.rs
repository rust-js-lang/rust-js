//@ run-fail: RefCell already borrowed
// A guard a `let` holds while code that borrows its cell runs meets that
// borrow, so its cell's borrows are counted, as std's are (ADRs 0328,
// 0863): a value with a destructor dropped while it's held, which borrows
// the cell mutably, panics.

use std::cell::RefCell;

struct Pushes<'a>(&'a RefCell<Vec<i32>>);

impl Drop for Pushes<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(0);
    }
}

fn main() {
    let cell = RefCell::new(vec![1]);
    {
        let v = cell.borrow();
        let n = v.len();
        println!("{n}");
        let _p = Pushes(&cell);
    }
    println!("{}", cell.borrow().len());
}
