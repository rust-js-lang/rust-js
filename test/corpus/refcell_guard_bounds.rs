// A `RefCell`'s `borrow_mut()` guard (#1815): while it lives, no other
// borrow of its cell can be made, so a call that may run anything, `f` of
// `take_first`, can't change what the cell holds, and the guard's length
// is kept past it. Not past what the guard's `&mut` is given to, its
// `truncate` or a function of the crate's, nor a write through it, nor a
// call once it's dropped, which may clear the cell through another `Rc`;
// nor of a `RefMut` a function gives of another cell. Each index is
// checked as Rust's panics say.
use std::any::Any;
use std::cell::{RefCell, RefMut};
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

/// A guard held in a variable keeps its cell's length past a call, but
/// not past what its `&mut` is given to (#1815): its `truncate`, a
/// function of the crate's, a write through it.
fn truncated_by_its_guard(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        items.truncate(0);
        items[i] = 1;
    }
    0
}

fn shrink(items: &mut Vec<u32>) {
    items.clear();
}

fn shrunk_by_a_call(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        shrink(&mut items);
        return items[i];
    }
    0
}

fn replaced_through_its_guard(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let mut items = queue.borrow_mut();
    if i < items.len() {
        *items = Vec::new();
        return items[i];
    }
    0
}

/// A `RefMut` a function gives, of another cell than the one it's given.
fn elsewhere<'a>(_queue: &'a RefCell<Vec<u32>>, other: &'a RefCell<Vec<u32>>) -> RefMut<'a, Vec<u32>> {
    other.borrow_mut()
}

fn borrowed_elsewhere(queue: &RefCell<Vec<u32>>, other: &RefCell<Vec<u32>>, i: usize) -> u32 {
    if i < queue.borrow().len() {
        let mut items = elsewhere(queue, other);
        items[i] = 1;
    }
    0
}

/// A guard dropped, then a call that clears the cell through another `Rc`
/// of it: the index checked again.
fn after_its_guard(queue: &RefCell<Vec<u32>>, clear: &dyn Fn(), i: usize) -> u32 {
    {
        let items = queue.borrow_mut();
        if i >= items.len() {
            return 0;
        }
    }
    clear();
    queue.borrow()[i]
}

/// A guard held across a call that may run anything: the cell's length
/// kept, as no other borrow of it can be made meanwhile.
fn take_first(queue: &RefCell<Vec<u32>>, f: &dyn Fn(u32) -> bool) -> Option<u32> {
    let mut items = queue.borrow_mut();
    for i in 0..items.len() {
        if f(items[i]) {
            return Some(items.remove(i));
        }
    }
    None
}

fn main() {
    let queue = Rc::new(RefCell::new(vec![11, 12]));
    check("truncated by its guard", || truncated_by_its_guard(&queue, 1));
    queue.borrow_mut().extend([13, 14]);
    check("shrunk by a call", || shrunk_by_a_call(&queue, 1));
    queue.borrow_mut().extend([15, 16]);
    check("replaced through its guard", || replaced_through_its_guard(&queue, 1));
    queue.borrow_mut().extend([17, 18]);
    let other = RefCell::new(vec![1]);
    check("borrowed elsewhere", || borrowed_elsewhere(&queue, &other, 1));
    check("borrowed elsewhere in bounds", || borrowed_elsewhere(&queue, &other, 0));
    let cleared = Rc::clone(&queue);
    check("after its guard", || after_its_guard(&queue, &|| cleared.borrow_mut().clear(), 1));
    queue.borrow_mut().extend([19, 20, 21]);
    let seen = Rc::clone(&queue);
    check("take first", || take_first(&queue, &|x| x > 19 && seen.try_borrow().is_err()));
    check("take none", || take_first(&queue, &|_| false));
    println!("queue: {:?}, other: {:?}", queue.borrow(), other.borrow());
}
