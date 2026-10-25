// A field's `RefCell` only ever borrowed for a moment is the value it holds
// (ADR 0362), but one a derive reads, which borrows it to show, clone and
// compare it, and one a pattern binds, a cell of its own then, lent, are
// cells: each shown, cloned, compared and borrowed as native Rust does.

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
struct Counter {
    hits: RefCell<u32>,
}

struct Wrap {
    n: RefCell<u16>,
}

fn bump(cell: &RefCell<u16>) -> u16 {
    *cell.borrow_mut() += 10;
    *cell.borrow()
}

fn main() {
    let counter = Counter {
        hits: RefCell::new(1),
    };
    *counter.hits.borrow_mut() += 1;
    let copy = counter.clone();
    *counter.hits.borrow_mut() += 1;
    println!("{:?} {:?} {}", counter, copy, counter == copy);
    let wrap = Wrap {
        n: RefCell::new(5),
    };
    *wrap.n.borrow_mut() += 1;
    let Wrap { n: lent } = &wrap;
    println!("{}", bump(lent));
    let Wrap { n } = wrap;
    *n.borrow_mut() += 1;
    let before = *n.borrow();
    println!("{before} {}", bump(&n));
    let again = Wrap { n };
    println!("{}", again.n.borrow());
}
