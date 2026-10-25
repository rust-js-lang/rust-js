//@ run-fail: RefCell already borrowed
// A field's `RefCell` a pattern binds is a cell of its own, whose borrows
// are counted: one held through the binding while the field is borrowed
// again finds it borrowed, as std's does (ADRs 0362, 0863).

use std::cell::RefCell;

struct Wrap {
    n: RefCell<u16>,
}

fn bump(cell: &RefCell<u16>) {
    *cell.borrow_mut() += 1;
}

fn main() {
    let wrap = Wrap { n: RefCell::new(1) };
    *wrap.n.borrow_mut() += 1;
    let Wrap { n: lent } = &wrap;
    bump(lent);
    println!("{}", wrap.n.borrow());
    let held = lent.borrow();
    *wrap.n.borrow_mut() += 1;
    println!("{}", *held);
}
