// An `Rc`'s `Deref` of a local the body doesn't reassign or lend `&mut`
// (#1839): the same `RefCell` each time, and it runs no code, so a length
// read through one `borrow()` of it is the next's: React's
// `listeners.borrow()[i]` under `i < listeners.borrow().len()`, and
// `deletions.borrow()[i]` of a `&Rc`. Not past a call that clears the
// cell through another `Rc` of it, nor once the local holds another
// `Rc`, nor past its `&mut` given to a call; nor of a `Box`'s cell, or an
// `Rc` of an `Rc`'s, a closure that clears it called between; nor of a
// `Deref` of the crate's, which may give another cell each time. Each
// index is checked as Rust's panics say.
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

/// React's listeners, called in order, each read again from the cell.
fn notified(listeners: Rc<RefCell<Vec<u32>>>) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < listeners.borrow().len() {
        total += listeners.borrow()[i];
        i += 1;
    }
    total
}

/// React's deletions, a `&Rc` of them.
fn deleted(deletions: &Rc<RefCell<Vec<u32>>>) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < deletions.borrow().len() {
        total += deletions.borrow()[i];
        i += 1;
    }
    total
}

/// A call between that clears the cell through another `Rc` of it.
fn cleared_between(listeners: &Rc<RefCell<Vec<u32>>>, clear: &dyn Fn()) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < listeners.borrow().len() {
        if i == 1 {
            clear();
        }
        total += listeners.borrow()[i];
        i += 1;
    }
    total
}

/// The local made another `Rc` between the test and the index.
fn reassigned(first: &Rc<RefCell<Vec<u32>>>, other: &Rc<RefCell<Vec<u32>>>, i: usize) -> u32 {
    let mut items = first.clone();
    if i < items.borrow().len() {
        items = other.clone();
        return items.borrow()[i];
    }
    0
}

fn replace(items: &mut Rc<RefCell<Vec<u32>>>, other: &Rc<RefCell<Vec<u32>>>) {
    *items = other.clone();
}

/// Its `&mut` given to a call between.
fn lent_between(first: &Rc<RefCell<Vec<u32>>>, other: &Rc<RefCell<Vec<u32>>>, i: usize) -> u32 {
    let mut items = first.clone();
    if i < items.borrow().len() {
        replace(&mut items, other);
        return items.borrow()[i];
    }
    0
}

/// A `Box`'s cell, a closure that clears it called between.
fn boxed(i: usize) -> u32 {
    let items = Box::new(RefCell::new(vec![1, 2, 3]));
    let clear = || items.borrow_mut().clear();
    if i < items.borrow().len() {
        clear();
        return items.borrow()[i];
    }
    0
}

/// An `Rc` of an `Rc`'s cell, the same.
fn doubled(i: usize) -> u32 {
    let items = Rc::new(Rc::new(RefCell::new(vec![1, 2, 3])));
    let clear = || items.borrow_mut().clear();
    if i < items.borrow().len() {
        clear();
        return items.borrow()[i];
    }
    0
}

/// A `Deref` of the crate's, which gives the other cell each time.
struct Alternating {
    cells: [RefCell<Vec<u32>>; 2],
    next: Cell<usize>,
}

impl Deref for Alternating {
    type Target = RefCell<Vec<u32>>;
    fn deref(&self) -> &RefCell<Vec<u32>> {
        let n = self.next.get();
        self.next.set(1 - n);
        &self.cells[n]
    }
}

fn alternating(cells: &Alternating, i: usize) -> u32 {
    if i < cells.borrow().len() {
        return cells.borrow()[i];
    }
    0
}

fn main() {
    check("notified", || notified(Rc::new(RefCell::new(vec![1, 2, 3]))));
    check("deleted", || deleted(&Rc::new(RefCell::new(vec![4, 5]))));
    let listeners = Rc::new(RefCell::new(vec![1, 2, 3]));
    let other = listeners.clone();
    check("cleared between", || cleared_between(&listeners, &|| other.borrow_mut().clear()));
    let first = Rc::new(RefCell::new(vec![1, 2, 3]));
    let short = Rc::new(RefCell::new(vec![9]));
    check("reassigned, in both", || reassigned(&first, &short, 0));
    check("reassigned", || reassigned(&first, &short, 2));
    check("lent between, in both", || lent_between(&first, &short, 0));
    check("lent between", || lent_between(&first, &short, 2));
    check("boxed, outside", || boxed(3));
    check("boxed", || boxed(1));
    check("doubled, outside", || doubled(3));
    check("doubled", || doubled(1));
    let cells = Alternating {
        cells: [RefCell::new(vec![1, 2, 3]), RefCell::new(vec![4])],
        next: Cell::new(0),
    };
    check("alternating, in both", || alternating(&cells, 0));
    check("alternating", || alternating(&cells, 1));
}
