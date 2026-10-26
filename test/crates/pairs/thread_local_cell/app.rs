//! A crate whose generic code makes a `RefCell<T>` of any value, a
//! `{ value }`, and whose thread-locals hold a `RefCell` of the type its
//! library shares as the array, as React's `ReactFiberStack` does: each
//! thread-local is a place of its own, its module's variable, the array
//! (ADR 0863, amended).
use std::cell::RefCell;

pub struct StackCursor<T> {
    pub current: RefCell<T>,
}

thread_local! {
    static VALUE_STACK: RefCell<Vec<Option<&'static str>>> = const { RefCell::new(Vec::new()) };
    static NAMES: RefCell<Vec<Option<&'static str>>> = RefCell::new(vec![Some("first")]);
}

pub fn create_cursor<T>(default_value: T) -> StackCursor<T> {
    StackCursor {
        current: RefCell::new(default_value),
    }
}

pub fn cell_of<T>(value: T) -> RefCell<T> {
    RefCell::new(value)
}

pub fn main() {
    VALUE_STACK.with_borrow_mut(|value_stack| value_stack.push(Some("a")));
    VALUE_STACK.with_borrow_mut(|value_stack| value_stack[0] = None);
    NAMES.with_borrow_mut(|names| names.push(None));
    let cursor = create_cursor(1);
    let cell = cell_of("x");
    println!(
        "{:?} {:?} {} {}",
        VALUE_STACK.with_borrow(|value_stack| value_stack.clone()),
        NAMES.with_borrow(|names| names.clone()),
        cursor.current.borrow(),
        cell.borrow()
    );
}
