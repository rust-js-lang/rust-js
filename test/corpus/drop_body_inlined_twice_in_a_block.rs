// A `Drop` body inlined where each of two values is dropped in one block,
// its temporary suggested the same word in each copy, `const value =
// INDEX`: each copy's own variable, spelled apart from the other's,
// `value` and `value$1`, never one name declared twice (rustc's
// drop/issue-48962.rs's shape; #1681 change 3, step 2's regression).
use std::cell::{Cell, RefCell};

thread_local! {
    static ORDER: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
    static INDEX: Cell<usize> = const { Cell::new(0) };
}

struct Dropee(usize);

impl Drop for Dropee {
    fn drop(&mut self) {
        ORDER.with_borrow_mut(|order| order[INDEX.get()] = self.0);
        INDEX.set(INDEX.get() + 1);
    }
}

fn sentinel() {
    ORDER.with_borrow_mut(|order| order[INDEX.get()] = 9);
    INDEX.set(INDEX.get() + 1);
}

fn main() {
    let mut x = Box::new(Dropee(1));
    drop(*x);
    sentinel();
    *x = Dropee(3);
    drop(x);
    ORDER.with_borrow(|order| println!("{order:?}"));
}
