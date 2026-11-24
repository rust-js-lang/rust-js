// A `RefCell`'s `BorrowError` and `BorrowMutError` shown by `Debug` are
// their names alone, as std writes them, plain and `{:#?}`, in a `Result`,
// through generic code's `Debug` and written into a `String` (#2476).
use std::cell::RefCell;
use std::fmt::{Debug, Write};

fn shown<T: Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn main() {
    let x = RefCell::new(1);
    let _g = x.borrow_mut();
    println!("{:?} | {:?}", x.try_borrow().map(|v| *v), x.try_borrow_mut().map(|v| *v));
    let shared = x.try_borrow().unwrap_err();
    let mutable = x.try_borrow_mut().unwrap_err();
    println!("{shared:?} {mutable:?}");
    println!("{shared:#?} {mutable:#?}");
    println!("{:#?}", x.try_borrow_mut().map(|v| *v));
    println!("{} {}", shown(&shared), shown(&mutable));
    let mut out = String::new();
    write!(out, "{shared:?}/{mutable:?}").unwrap();
    println!("{out}");
    println!("{shared} | {mutable}");
}
