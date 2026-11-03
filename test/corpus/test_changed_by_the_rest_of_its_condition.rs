// A test an `&&` or an `||` changes before its condition ends (ADR 0576,
// amended): `i < v.len() && { v.clear(); true }` shows nothing of `i` in
// its branch, as the `clear` came after the test, so `v[i]` is checked
// there and panics as Rust's does; and of a `RefCell`'s `Vec`, cleared by
// a call on the right, `i < queue.borrow().len() && clear(queue)` (ADR
// 0881). Where nothing on the right changes it, the test shows it still.
use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

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

fn clear(queue: &RefCell<Vec<u32>>) -> bool {
    queue.borrow_mut().clear();
    true
}

fn cleared_on_the_right(v: &mut Vec<u32>, i: usize) -> u32 {
    if i < v.len()
        && {
            v.clear();
            true
        }
    {
        v[i]
    } else {
        7
    }
}

fn cell_cleared_on_the_right(queue: &RefCell<Vec<u32>>, i: usize) -> u32 {
    if i < queue.borrow().len() && clear(queue) {
        queue.borrow()[i]
    } else {
        7
    }
}

fn cleared_in_a_group(v: &mut Vec<u32>, i: usize) -> u32 {
    if (i < v.len()
        && {
            v.clear();
            true
        })
        && i < 10
    {
        v[i]
    } else {
        7
    }
}

fn cleared_past_an_or(v: &mut Vec<u32>, i: usize) -> u32 {
    if i >= v.len()
        || {
            v.clear();
            false
        }
    {
        7
    } else {
        v[i]
    }
}

fn kept_on_the_right(v: &[u32], i: usize) -> u32 {
    if i < v.len() && v[i] > 1 { v[i] } else { 7 }
}

fn main() {
    check("cleared on the right", || cleared_on_the_right(&mut vec![1, 2, 3], 1));
    let queue = RefCell::new(vec![1, 2, 3]);
    check("cell cleared on the right", || cell_cleared_on_the_right(&queue, 1));
    check("cleared in a group", || cleared_in_a_group(&mut vec![1, 2, 3], 2));
    check("cleared past an or", || cleared_past_an_or(&mut vec![1, 2, 3], 0));
    check("cleared past an or, out", || cleared_past_an_or(&mut vec![1, 2, 3], 5));
    println!("kept on the right: {}", kept_on_the_right(&[1, 2, 3], 2));
    println!("kept on the right, small: {}", kept_on_the_right(&[1, 2, 3], 0));
}
