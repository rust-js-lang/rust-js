//@ run-fail: RefCell already mutably borrowed
// A cell a trait's method reaches through a `dyn` is any cell of its type
// that code reaches (ADR 0863, amended): the method may be called by code
// the crate doesn't see, given what it doesn't know, so a borrow it takes
// while one of the same cell is held is checked, and panics, as std's does.

use std::cell::RefCell;

trait Touch {
    fn touch(&self) -> usize;
}

struct Holder {
    cell: RefCell<Vec<i32>>,
}

impl Touch for Holder {
    fn touch(&self) -> usize {
        self.cell.borrow().len()
    }
}

fn run(touch: &dyn Touch) -> usize {
    touch.touch()
}

fn main() {
    let holder = Holder {
        cell: RefCell::new(vec![1]),
    };
    println!("{}", run(&holder));
    let mut held = holder.cell.borrow_mut();
    held.push(2);
    println!("{}", run(&holder));
    println!("{}", held.len());
}
