// An `Rc<T>` only an impl's header names, `impl<T> StableDeref for Rc<T>`,
// whose dictionary holds `Rc<T>`'s `Deref` and drop: where the crate counts
// another `Rc`, an `Rc<dyn Fn>` whose `ptr_eq` it asks of a cloned closure
// (ADR 0959), `T` may be that one, so every `Rc` given the impl is counted
// too, as generic code's is (ADR 0852). Counted there and not where it's
// made, the `Rc<Vec<u32>>` read its `.value` of a plain array (#949).
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// # Safety
/// Its target stays put while it's moved.
pub unsafe trait StableDeref: Deref {}

unsafe impl<T: ?Sized> StableDeref for Rc<T> {}

fn total<D: StableDeref<Target = Vec<u32>>>(items: D) -> u32 {
    items.iter().sum()
}

fn cloned(count: &Rc<RefCell<u32>>) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let count = Rc::clone(count);
    let read = move || *count.borrow();
    (Rc::new(read.clone()), Rc::new(read))
}

fn main() {
    println!("{}", total(Rc::new(vec![5, 6])));
    let count = Rc::new(RefCell::new(4));
    let (first, second) = cloned(&count);
    println!("{} {} {}", first(), second(), Rc::ptr_eq(&first, &second));
}
