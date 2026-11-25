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

pub fn copied(slot: &RefCell<Option<f64>>) -> f64 {
    // A value copied out, read past what writes the cell (#2579).
    let Some(id) = *slot.borrow() else {
        return 0.0;
    };
    *slot.borrow_mut() = None;
    id
}

#[derive(Clone)]
pub struct Id(pub f64);

#[derive(Clone)]
pub struct Lease(pub f64);

impl Drop for Lease {
    fn drop(&mut self) {
        println!("lease {} ended", self.0);
    }
}

thread_local! {
    static CURRENT_ID: Rc<RefCell<Option<Id>>> = Rc::new(RefCell::new(Some(Id(4.0))));
    static LEASE: Rc<RefCell<Option<Lease>>> = Rc::new(RefCell::new(Some(Lease(6.0))));
}

fn forget_id() {
    CURRENT_ID.with(|id| *id.borrow_mut() = None);
}

fn end_lease() {
    LEASE.with(|lease| *lease.borrow_mut() = None);
}

pub fn forgotten() -> f64 {
    // Read just past a call given nothing, which writes the cell.
    let slot = CURRENT_ID.with(Rc::clone);
    let Some(id) = slot.borrow().clone() else {
        return 0.0;
    };
    forget_id();
    id.0
}

pub fn held_to_the_end() -> bool {
    // Read only by its drop, past a call that writes the cell.
    let slot = LEASE.with(Rc::clone);
    let Some(_held) = slot.borrow().clone() else {
        return false;
    };
    end_lease();
    true
}

fn main() {
    let _never_called: fn(&Handler) -> f64 = held;
    let handler = Handler { chunk: RefCell::new(Some(Rc::new(Chunk { id: 5.0 }))) };
    println!("{}", first(&handler));
    *handler.chunk.borrow_mut() = Some(Rc::new(Chunk { id: 7.0 }));
    println!("{}", second(&handler));
    println!("{}", lend(&handler.chunk));
    println!("{}", copied(&RefCell::new(Some(3.0))));
    println!("{}", forgotten());
    println!("{}", held_to_the_end());
}
