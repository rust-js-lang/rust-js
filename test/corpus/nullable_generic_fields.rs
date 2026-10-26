//@ run-pass
// A generic struct's `#[rust_js::nullable]` field of a type parameter's
// value, and a `#[rust_js::nullable]` thread-local a generic constructor
// makes, as React's `StackCursor`s are: where the type parameter is an
// `Option`, its `None` is `null` where it's made and written, which every
// read takes for `None` as Rust's does (ADR 0586, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct StackCursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current: RefCell<T>,
}

pub struct Dependency<T: 'static> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub memoized_value: Cell<T>,
}

pub fn create_cursor<T>(default_value: T) -> StackCursor<T> {
    StackCursor {
        current: RefCell::new(default_value),
    }
}

pub fn push<T>(cursor: &StackCursor<T>, value: T, stack: &mut Vec<T>) {
    stack.push(cursor.current.replace(value));
}

pub fn pop<T>(cursor: &StackCursor<T>, stack: &mut Vec<T>) {
    if let Some(previous) = stack.pop() {
        cursor.current.replace(previous);
    }
}

pub struct Hidden {
    pub lanes: u32,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static HIDDEN: StackCursor<Option<Rc<Hidden>>> = create_cursor(None);
}

fn hidden_lanes() -> Option<u32> {
    HIDDEN.with(|cursor| cursor.current.borrow().as_ref().map(|hidden| hidden.lanes))
}

fn main() {
    let mut stack = Vec::new();
    println!("{:?}", hidden_lanes());
    HIDDEN.with(|cursor| push(cursor, Some(Rc::new(Hidden { lanes: 4 })), &mut stack));
    println!("{:?} {}", hidden_lanes(), stack.len());
    HIDDEN.with(|cursor| pop(cursor, &mut stack));
    println!("{:?} {}", hidden_lanes(), HIDDEN.with(|cursor| cursor.current.borrow().is_none()));

    let cursor = StackCursor {
        current: RefCell::new(None::<u8>),
    };
    let was = cursor.current.replace(Some(3));
    println!("{was:?} {:?}", cursor.current.borrow());
    *cursor.current.borrow_mut() = None;
    println!("{:?} {}", cursor.current.borrow(), *cursor.current.borrow() == None);

    let dependency = Dependency {
        memoized_value: Cell::new(Some(1)),
    };
    dependency.memoized_value.set(None);
    let numbers = Dependency {
        memoized_value: Cell::new(2u32),
    };
    match dependency.memoized_value.get() {
        Some(value) => println!("some {value}"),
        None => println!("none {}", numbers.memoized_value.get()),
    }
    println!("{}", dependency.memoized_value.get().unwrap_or(9));
}
