// Callbacks that may read a cell before it's set, so before the call whose
// value it holds returns: Rust's `unwrap` of it panics there, where a
// variable not yet made would throw another error, and one already made
// would give the value. Each cell is a cell, as written, and its `unwrap`
// panics as Rust's does (ADR 0317, amended, rd621): of a callback its
// executor calls, through the variable it moved it into; one the executor
// gives to a call, which calls it; one moved into its maker's caller's
// place, which calls it once the call has panicked; and one the maker
// calls before the set. Nor is a cell of a value read changed in place, or
// whose variable is changed after, its variable.

use std::any::Any;
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn make(name: &'static str, executor: impl FnOnce()) -> &'static str {
    executor();
    name
}

fn keep(callback: Rc<dyn Fn(u32)>) -> Option<Rc<dyn Fn(u32)>> {
    callback(0);
    Some(callback)
}

fn called_at_once(name: &'static str) -> &'static str {
    let mut resolve: Option<Rc<dyn Fn(u32)>> = None;
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let handle = make(name, || {
        let resolved = Rc::clone(&wrapper);
        resolve = Some(Rc::new(move |n| println!("{} resolved with {n}", resolved.get().unwrap())));
        if let Some(resolve) = &resolve {
            resolve(0);
        }
    });
    wrapper.set(Some(handle));
    if let Some(resolve) = resolve {
        resolve(1);
    }
    handle
}

fn given_on(name: &'static str) -> &'static str {
    let mut resolve: Option<Rc<dyn Fn(u32)>> = None;
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let handle = make(name, || {
        let resolved = Rc::clone(&wrapper);
        resolve = keep(Rc::new(move |n| println!("{} resolved with {n}", resolved.get().unwrap())));
    });
    wrapper.set(Some(handle));
    if let Some(resolve) = resolve {
        resolve(1);
    }
    handle
}

fn failing(_: &'static str, executor: impl FnOnce()) -> &'static str {
    executor();
    panic!("no handle")
}

fn stored(name: &'static str, kept: &mut Option<Rc<dyn Fn()>>) -> &'static str {
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let handle = failing(name, || {
        let resolved = Rc::clone(&wrapper);
        *kept = Some(Rc::new(move || println!("{} kept", resolved.get().unwrap())));
    });
    wrapper.set(Some(handle));
    handle
}

fn called_before_set(name: &'static str) -> &'static str {
    let mut resolve: Option<Rc<dyn Fn(u32)>> = None;
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let handle = make(name, || {
        let resolved = Rc::clone(&wrapper);
        resolve = Some(Rc::new(move |n| println!("{} resolved with {n}", resolved.get().unwrap())));
    });
    if let Some(resolve) = &resolve {
        resolve(0);
    }
    wrapper.set(Some(handle));
    handle
}

#[derive(Clone, Copy)]
struct Point {
    x: u32,
}

fn point(executor: impl FnOnce()) -> Point {
    executor();
    Point { x: 1 }
}

fn changed_in_place() -> u32 {
    let mut moved: Option<Rc<dyn Fn()>> = None;
    let wrapper: Rc<Cell<Option<Point>>> = Rc::default();
    let handle = point(|| {
        let read = Rc::clone(&wrapper);
        moved = Some(Rc::new(move || {
            let mut p = read.get().unwrap();
            p.x += 10;
            println!("moved to {}", p.x);
        }));
    });
    wrapper.set(Some(handle));
    if let Some(moved) = moved {
        moved();
    }
    handle.x
}

fn changed_after(name: &'static str) -> &'static str {
    let mut resolve: Option<Rc<dyn Fn(u32)>> = None;
    let wrapper: Rc<Cell<Option<&'static str>>> = Rc::default();
    let mut handle = make(name, || {
        let resolved = Rc::clone(&wrapper);
        resolve = Some(Rc::new(move |n| println!("{} resolved with {n}", resolved.get().unwrap())));
    });
    wrapper.set(Some(handle));
    let changed = &mut handle;
    *changed = "changed";
    if let Some(resolve) = resolve {
        resolve(1);
    }
    handle
}

fn message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast_ref::<&str>() {
        Some(s) => s.to_string(),
        None => "another payload".to_string(),
    }
}

fn check(what: &str, f: impl FnOnce() -> String) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(handle) => println!("{what}: {handle}"),
        Err(payload) => println!("{what} panicked: {}", message(payload)),
    }
}

fn main() {
    check("at once", || called_at_once("a").to_string());
    check("given on", || given_on("b").to_string());
    let mut kept = None;
    check("stored", || stored("c", &mut kept).to_string());
    check("kept", || {
        if let Some(kept) = kept {
            kept();
        }
        "after".to_string()
    });
    check("before", || called_before_set("d").to_string());
    check("in place", || changed_in_place().to_string());
    check("after", || changed_after("e").to_string());
}
