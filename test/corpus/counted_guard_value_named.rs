// A temporary a `const` takes apart, the value read out of a run-time
// tracked cell's guard, `const { value: _4 } = $borrow($fieldCell(..))`,
// is named after the variable the program gives it, `let Some(chunk) =
// handler.chunk.borrow().clone() else ..`'s `chunk`, as React's Flight
// client's initializeChunkIfUnblocked writes it (rd439).
use std::cell::RefCell;
use std::rc::Rc;

pub struct Chunk {
    pub id: f64,
    pub state: RefCell<State>,
}

pub enum State {
    Blocked,
    Done(f64),
}

pub struct Handler {
    #[cfg_attr(rust_js, rust_js::nullable)]
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
    if !matches!(*chunk.state.borrow(), State::Blocked) {
        return 1.0;
    }
    touch(handler);
    settle(&chunk);
    chunk.id
}

fn settle(chunk: &Rc<Chunk>) {
    *chunk.state.borrow_mut() = State::Done(chunk.id);
}

fn lend(cell: &RefCell<Option<Rc<Chunk>>>) -> bool {
    cell.borrow().is_some()
}

pub fn held(handler: &Handler) -> f64 {
    let guard = handler.chunk.borrow();
    touch(handler);
    if lend(&handler.chunk) { guard.as_ref().map_or(0.0, |c| c.id) } else { 1.0 }
}

fn main() {
    let _never_called: fn(&Handler) -> f64 = held;
    let handler = Handler { chunk: RefCell::new(Some(Rc::new(Chunk { id: 5.0, state: RefCell::new(State::Blocked) }))) };
    println!("{}", first(&handler));
    println!("{}", lend(&handler.chunk));
}
