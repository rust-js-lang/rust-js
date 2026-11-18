// A `RefCell`'s guard held across code that writes its cell through
// another `Rc` of it (#2112 B): Rust's second borrow panics, so what's
// known of the guard's length holds past the code wherever it runs on, and
// JS must panic where Rust does, never read past a length the code
// changed. Under a `borrow()` and under a `borrow_mut()`, the code a
// closure the crate's own, given as `impl Fn` or as `&dyn Fn`, and code
// that only reads through the clone, which runs on; and past a guard
// dropped, the length known before it no longer so.
use std::any::Any;
use std::cell::RefCell;
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

/// Read under a `borrow()` past code the crate's own.
fn read_under(items: &RefCell<Vec<u32>>, f: impl Fn(), i: usize) -> u32 {
    let held = items.borrow();
    if i < held.len() {
        f();
        return held[i];
    }
    0
}

/// Written under a `borrow_mut()` past code the crate's own.
fn write_under(items: &RefCell<Vec<u32>>, f: impl Fn(), i: usize) -> u32 {
    let mut held = items.borrow_mut();
    if i < held.len() {
        f();
        held[i] += 1;
        return held[i];
    }
    0
}

/// Read under a `borrow()` past code behind a `dyn`.
fn read_under_dyn(items: &RefCell<Vec<u32>>, f: &dyn Fn(), i: usize) -> u32 {
    let held = items.borrow();
    if i < held.len() {
        f();
        return held[i];
    }
    0
}

/// A guard dropped, then code that clears the cell through a clone: the
/// length the guard showed is no longer so, and the index panics as Rust's
/// does.
fn after_a_dropped_guard(items: &RefCell<Vec<u32>>, f: &dyn Fn(), i: usize) -> u32 {
    if i >= items.borrow().len() {
        return 0;
    }
    {
        let held = items.borrow();
        let _length = held.len();
    }
    f();
    items.borrow()[i]
}

/// A cell a parameter's struct holds, read past code that clears it through
/// a clone, no guard held: the index panics as Rust's does.
struct Queue {
    items: Rc<RefCell<Vec<u32>>>,
}

fn field_after_code(queue: &Queue, f: &dyn Fn(), i: usize) -> u32 {
    if i < queue.items.borrow().len() {
        f();
        return queue.items.borrow()[i];
    }
    0
}

fn main() {
    let items = Rc::new(RefCell::new(vec![1, 2, 3]));
    let clone = Rc::clone(&items);
    check("read, nothing written", || read_under(&items, || {}, 2));
    check("read, cleared through a clone", || read_under(&items, || clone.borrow_mut().clear(), 2));
    check("read, read through a clone", || read_under(&items, || println!("{}", clone.borrow().len()), 2));
    check("write, read through a clone", || write_under(&items, || println!("{}", clone.try_borrow().is_err()), 2));
    check("write, cleared through a clone", || write_under(&items, || clone.borrow_mut().clear(), 2));
    check("read dyn, cleared through a clone", || read_under_dyn(&items, &|| clone.borrow_mut().clear(), 2));
    items.borrow_mut().extend([4, 5, 6]);
    check("after a dropped guard, cleared through a clone", || after_a_dropped_guard(&items, &|| clone.borrow_mut().clear(), 1));
    items.borrow_mut().extend([7, 8, 9]);
    let queue = Queue { items: Rc::clone(&items) };
    check("a field's cell, cleared through a clone", || field_after_code(&queue, &|| clone.borrow_mut().clear(), 1));
    println!("items: {:?}", items.borrow());
}
