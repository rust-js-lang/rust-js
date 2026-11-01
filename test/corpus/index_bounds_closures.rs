// An index below two slices' lengths, `i < prev.len() && i < next.len()`,
// is in bounds of each, `next[i]` and `prev[i]`, as React's
// areHookInputsEqual reads them; and so in a closure that captures what
// it reads by reference, `OBJECT_IS.with(|is| is(&next[i], &prev[i]))`,
// which Rust's borrows keep as they were where it's made. Where the
// closure may change what it captured, or reads a slice the test didn't
// bound, each index is checked, and panics as Rust's does.
use std::cell::{Cell, RefCell};

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
}

fn same(a: &u32, b: &u32) -> bool {
    CALLS.set(CALLS.get() + 1);
    a == b
}

fn equal(next: &[u32], prev: &[u32]) -> bool {
    let mut i = 0;
    while i < prev.len() && i < next.len() {
        if !same(&next[i], &prev[i]) {
            return false;
        }
        i += 1;
    }
    true
}

fn equal_in_closure(next: &[u32], prev: &[u32]) -> bool {
    let mut i = 0;
    while i < prev.len() && i < next.len() {
        if CALLS.with(|_| same(&next[i], &prev[i])) {
            i += 1;
            continue;
        }
        return false;
    }
    true
}

fn nested(a: &[u32], b: &[u32]) -> u32 {
    let mut sum = 0;
    for i in 0..a.len() {
        if i < b.len() {
            sum += a[i] * b[i];
        }
    }
    sum
}

fn later(items: &[u32], i: usize) -> u32 {
    if i < items.len() {
        let read = || items[i] + 1;
        read() + read()
    } else {
        0
    }
}

fn in_a_nested_closure(items: &[u32], i: usize) -> u32 {
    if i < items.len() {
        let outer = || {
            let inner = || items[i];
            inner() * 2
        };
        outer()
    } else {
        0
    }
}

fn another_slice(a: &[u32], b: &[u32], i: usize) -> u32 {
    if i < a.len() {
        let read = || b[i];
        read()
    } else {
        0
    }
}

fn changed_by_the_closure(items: &[u32]) -> Vec<u32> {
    let mut i = 0;
    let mut found = Vec::new();
    if i < items.len() {
        let mut step = move || {
            let item = items[i];
            i += 1;
            item
        };
        while found.len() <= items.len() {
            found.push(step());
        }
    }
    found
}

fn changed_through_a_borrow(items: &[u32]) -> Vec<u32> {
    let mut i = 0;
    let mut found = Vec::new();
    if i < items.len() {
        let mut step = || {
            let item = items[i];
            i += 1;
            item
        };
        while found.len() <= items.len() {
            found.push(step());
        }
    }
    found
}

fn in_a_cell(cell: &RefCell<Vec<u32>>, i: usize) -> u32 {
    if i < cell.borrow().len() {
        let read = || cell.borrow()[i];
        cell.borrow_mut().clear();
        read()
    } else {
        0
    }
}

fn caught<T: std::fmt::Debug>(f: impl FnOnce() -> T + std::panic::UnwindSafe) {
    match std::panic::catch_unwind(f) {
        Ok(value) => println!("{value:?}"),
        Err(payload) => match payload.downcast_ref::<String>() {
            Some(message) => println!("{message}"),
            None => println!("a panic"),
        },
    }
}

fn main() {
    println!("{}", equal(&[1, 2, 3], &[1, 2, 3]));
    println!("{}", equal(&[1, 2, 3], &[1, 2]));
    println!("{}", equal(&[1, 5], &[1, 2, 3]));
    println!("{}", equal_in_closure(&[1, 2, 3], &[1, 2, 3]));
    println!("{}", equal_in_closure(&[1, 2], &[1, 2, 3]));
    println!("{}", equal_in_closure(&[4, 2], &[1, 2, 3]));
    println!("{}", CALLS.get());
    println!("{}", nested(&[1, 2, 3], &[4, 5]));
    println!("{}", later(&[1, 2, 3], 2));
    println!("{}", later(&[1, 2, 3], 3));
    println!("{}", in_a_nested_closure(&[1, 2, 3], 1));
    println!("{}", another_slice(&[1, 2, 3], &[7, 8, 9, 10], 2));
    caught(|| changed_by_the_closure(&[3, 4, 5]));
    caught(|| changed_through_a_borrow(&[3, 4, 5]));
    caught(|| in_a_cell(&RefCell::new(vec![1, 2]), 1));
    caught(|| another_slice(&[1, 2, 3], &[7], 2));
}
