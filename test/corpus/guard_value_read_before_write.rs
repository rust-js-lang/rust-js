// A value read out of a `RefCell` field's guard, `let Some(chunk) =
// handler.chunk.borrow().clone() else ..`, is read while the guard is,
// where Rust clones it, not where it's used after a call that writes the
// cell: React's Flight client's initializeChunkIfUnblocked (#452). The
// field is a cell tracked at run time, as `held` keeps a borrow across a
// call.
use std::cell::RefCell;
use std::rc::Rc;

pub struct Chunk {
    pub id: f64,
}

pub struct Handler {
    pub chunk: RefCell<Option<Rc<Chunk>>>,
}

pub fn touch(handler: &Handler) {
    *handler.chunk.borrow_mut() = None;
}

pub fn first(handler: &Handler) -> f64 {
    // The clone is taken, and the guard dropped, before `touch` clears the cell.
    let Some(chunk) = handler.chunk.borrow().clone() else {
        return 0.0;
    };
    touch(handler);
    chunk.id
}

fn lend(cell: &RefCell<Option<Rc<Chunk>>>) -> bool {
    cell.borrow().is_some()
}

pub fn held(handler: &Handler) -> f64 {
    let guard = handler.chunk.borrow();
    touch(handler);
    if lend(&handler.chunk) { guard.as_ref().map_or(0.0, |c| c.id) } else { 1.0 }
}

pub fn second(handler: &Handler) -> f64 {
    // Through a guard named, read then dropped.
    let guard = handler.chunk.borrow();
    let chunk = guard.clone();
    drop(guard);
    touch(handler);
    chunk.map_or(-1.0, |c| c.id)
}

fn main() {
    let _never_called: fn(&Handler) -> f64 = held;
    let handler = Handler { chunk: RefCell::new(Some(Rc::new(Chunk { id: 5.0 }))) };
    println!("{}", first(&handler));
    *handler.chunk.borrow_mut() = Some(Rc::new(Chunk { id: 7.0 }));
    println!("{}", second(&handler));
    println!("{}", lend(&handler.chunk));
}
