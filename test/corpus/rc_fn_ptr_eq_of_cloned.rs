// `Rc::ptr_eq` of `Rc<dyn Fn()>`s made of a closure that's cloned,
// `Rc::new(read.clone())` and `Rc::new(read)`: one JS function may be given
// it twice, so they're counted, `{ value, strong, weak }`, compared by their
// boxes, as Rust compares two `Rc`s (ADR 0959, amended).
use std::cell::RefCell;
use std::rc::Rc;

fn cloned(count: &Rc<RefCell<u32>>) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let count = Rc::clone(count);
    let read = move || *count.borrow();
    let first: Rc<dyn Fn() -> u32> = Rc::new(read.clone());
    let second: Rc<dyn Fn() -> u32> = Rc::new(read);
    (first, second)
}

fn main() {
    let count = Rc::new(RefCell::new(4));
    let (first, second) = cloned(&count);
    let again = Rc::clone(&first);
    println!("{} {} {} {}", first(), second(), Rc::ptr_eq(&first, &second), Rc::ptr_eq(&first, &again));
}
