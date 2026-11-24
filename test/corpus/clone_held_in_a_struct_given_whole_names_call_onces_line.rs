//@ run-fail: RefCell already mutably borrowed
// `RefCell::clone` held in a struct's field, the struct given whole to a
// function that moves the field into `Option::map`, which calls it through
// `FnOnce::call_once`: it panics at that shim's line, `function.rs:250:5`,
// as the callee's MIR uses that field, which no bound of the struct says
// (#2522).
use std::cell::RefCell;

struct Copier<F> {
    copy: F,
}

fn first_copy<T, F: FnMut(&T) -> T>(copier: Copier<F>, items: &[T]) -> Option<T> {
    items.first().map(copier.copy)
}

fn main() {
    let items = vec![RefCell::new(1u8)];
    let _held = items[0].borrow_mut();
    println!("{}", items.len());
    println!("{:?}", first_copy(Copier { copy: RefCell::clone }, &items));
}
