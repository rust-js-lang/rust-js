//@ run-fail: RefCell already borrowed
// A value with a destructor a `let` takes after a guard, `let q = p;`,
// moved from a place, drops where its block ends, before the guard does:
// its destructor runs while the guard is held, and one that borrows the
// cell mutably panics, as std's does (ADR 0863).

use std::cell::RefCell;

struct Pushes<'a>(&'a RefCell<Vec<i32>>);

impl Drop for Pushes<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(0);
    }
}

fn main() {
    let cell = RefCell::new(vec![1]);
    let p = Pushes(&cell);
    {
        let v = cell.borrow();
        println!("{}", v.len());
        let _q = p;
    }
    println!("{}", cell.borrow().len());
}
