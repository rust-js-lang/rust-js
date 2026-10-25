//@ run-fail: RefCell already borrowed
// A borrow held while std's code clones a `Vec` of the crate's types is
// held while their `Clone` runs, a derive's of what its fields' do: one
// that borrows the same cell mutably finds it borrowed, as std's does, so
// the cell's borrows are counted, though each is held for a moment (ADRs
// 0328, 0863).

use std::cell::RefCell;

struct Peek<'a>(&'a RefCell<Vec<i32>>);

impl Clone for Peek<'_> {
    fn clone(&self) -> Self {
        self.0.borrow_mut().push(0);
        Peek(self.0)
    }
}

#[derive(Clone)]
struct Holder<'a> {
    peek: Peek<'a>,
}

fn main() {
    let cell = RefCell::new(vec![1]);
    let peeks = vec![Peek(&cell)];
    let cloned = peeks.clone();
    println!("{} {}", cloned.len(), cell.borrow().len());
    let holders = vec![Holder { peek: Peek(&cell) }];
    let n = cell.borrow().len() + holders.clone().len();
    println!("{n}");
}
