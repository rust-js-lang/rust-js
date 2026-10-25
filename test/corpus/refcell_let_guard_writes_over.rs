//@ run-fail: RefCell already borrowed
// A write over a value with a destructor, `p = q;`, runs it: one that
// borrows the cell a `let`'s guard holds mutably, while it's held, panics,
// as std's does (ADR 0863).

use std::cell::RefCell;

struct Pushes<'a>(&'a RefCell<Vec<i32>>);

impl Drop for Pushes<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(0);
    }
}

fn main() {
    let cell = RefCell::new(vec![1]);
    let q = Pushes(&cell);
    let mut p = Pushes(&cell);
    {
        let v = cell.borrow();
        println!("{}", v.len());
        p = q;
    }
    println!("{} {}", cell.borrow().len(), p.0.borrow().len());
}
