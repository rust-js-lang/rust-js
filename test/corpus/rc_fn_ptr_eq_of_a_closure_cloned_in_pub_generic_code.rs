//@ library-differs
// `Rc::ptr_eq` of `Rc<dyn Fn()>`s made of a closure and of its clone made
// in `pub` generic code, which JS or a consumer may call too, `T` any type
// they give, none a closure of the crate's (#1250): an `Rc` of what that
// code gives back is made apart, so the two are two allocations, never
// equal, counted and compared by their boxes (ADR 0959, #1249).
use std::cell::RefCell;
use std::rc::Rc;

pub fn dup<T: Clone>(x: &T) -> T {
    x.clone()
}

pub fn twice<T: Clone>(x: T) -> (T, T) {
    (x.clone(), x)
}

fn cloned(count: &Rc<RefCell<u32>>) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let count = Rc::clone(count);
    let read = move || *count.borrow();
    let first = Rc::new(dup(&read));
    let second = Rc::new(read);
    (first, second)
}

fn pair() -> (Rc<dyn Fn(u32) -> u32>, Rc<dyn Fn(u32) -> u32>) {
    let (a, b) = twice(|x: u32| x + 1);
    (Rc::new(a), Rc::new(b))
}

/// A closure of generic code's own, cloned there: the crate clones it as
/// each instance its callers give.
fn shown<T: std::fmt::Debug + 'static>(x: Rc<T>) -> (Rc<dyn Fn() -> String>, Rc<dyn Fn() -> String>) {
    let show = move || format!("{x:?}");
    (Rc::new(show.clone()), Rc::new(show))
}

fn main() {
    let (s, t) = shown(Rc::new(7u8));
    println!("{} {} {}", s(), t(), Rc::ptr_eq(&s, &t));
    let count = Rc::new(RefCell::new(4));
    let (first, second) = cloned(&count);
    let again = Rc::clone(&first);
    println!(
        "{} {} {} {}",
        first(),
        second(),
        Rc::ptr_eq(&first, &second),
        Rc::ptr_eq(&first, &again)
    );
    let (a, b) = pair();
    println!("{} {} {}", a(1), b(2), Rc::ptr_eq(&a, &b));
}
