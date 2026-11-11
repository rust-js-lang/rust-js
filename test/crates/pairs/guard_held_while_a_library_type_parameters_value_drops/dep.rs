//! A library's generic code that holds a guard while a value the consumer
//! gives it drops, which its own crate never gives one with a destructor:
//! the consumer compiles its own (ADR 0914), from the library's MIR, which
//! has no THIR to say the guard is held while nothing asks, so each of its
//! borrows is counted, as the destructor asks of the cell (#1325).
use std::cell::RefCell;

pub fn push_after<T>(it: T, counts: &RefCell<Vec<u8>>) {
    counts.borrow_mut().push({
        let _it = it;
        1
    });
}

pub fn push_in_closure<T>(it: T, counts: &RefCell<Vec<i32>>) {
    let run = move || {
        counts.borrow_mut().push({
            let _it = it;
            2
        });
    };
    run();
}

/// A trait's default, which the consumer compiles for its own impl.
pub trait Pushed {
    fn push_self(self, counts: &RefCell<Vec<u16>>)
    where
        Self: Sized,
    {
        counts.borrow_mut().push({
            let _it = self;
            3
        });
    }
}
