// What JS made, read in closures and replaced: a getter's value a closure
// captures, read once where Rust reads it, not where the closure runs; a
// closure that reads it each time it's called; and a JS-made field
// replaced, read once before it's written (#536).
//@ call: captured [{"count": 4, "bump": {"$fn": null}}]
//@ call: eachCall [{"count": 4}]
//@ call: replaced [{"slot": {"n": 1}}, {"n": 2}]
//@ call: swapped [{"left": 1, "right": 2}]

use js::Unknown;
use std::cell::{Cell, RefCell};

pub trait Counter {
    #[cfg_attr(rust_js, rust_js::link_name = "get count")]
    fn count(&self) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "bump")]
    fn bump(&self) {
        unreachable!()
    }
}

pub struct Counted;

impl Counter for Counted {}

pub fn captured(counter: &Counted) -> u32 {
    let count = counter.count();
    let twice = move || count * 2;
    counter.bump();
    twice() + twice()
}

pub fn each_call(counter: &Counted) -> u32 {
    let read = || counter.count() + 1;
    read() + read()
}

pub struct Holder {
    pub slot: RefCell<&'static Unknown>,
}

// Safety: what JS gives is an object of this field.
unsafe impl js::Defined for Holder {}

pub fn replaced(holder: &Unknown, next: &'static Unknown) -> bool {
    // Safety: what's given is a holder.
    let holder: &Holder = unsafe { js::cast(Some(holder)) };
    let old = std::mem::replace(&mut *holder.slot.borrow_mut(), next);
    std::ptr::eq(old, next)
}

pub struct Pair {
    pub left: Cell<u32>,
    pub right: Cell<u32>,
}

// Safety: what JS gives is an object of these fields.
unsafe impl js::Defined for Pair {}

pub fn swapped(pair: &Unknown) -> u32 {
    // Safety: what's given is a pair.
    let pair: &Pair = unsafe { js::cast(Some(pair)) };
    pair.left.swap(&pair.right);
    pair.left.get() * 10 + pair.right.get()
}
