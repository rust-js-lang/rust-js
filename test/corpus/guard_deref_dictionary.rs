// std's `Deref` of a `Ref<T>`, in a dictionary, as stable_deref_trait's
// `StableDeref` takes it: the guard's value, as its `*` reads it (ADR 0451).
use std::cell::{Ref, RefCell, RefMut};
use std::ops::Deref;
use std::rc::Rc;

/// # Safety
/// Its target stays put while it's moved.
pub unsafe trait StableDeref: Deref {}

unsafe impl<T> StableDeref for Vec<T> {}
unsafe impl StableDeref for String {}
unsafe impl<T: ?Sized> StableDeref for Rc<T> {}
unsafe impl<T: ?Sized> StableDeref for Ref<'_, T> {}
unsafe impl<T: ?Sized> StableDeref for RefMut<'_, T> {}

fn total<D: StableDeref<Target = Vec<u32>>>(items: D) -> u32 {
    items.iter().sum()
}

fn length<D: StableDeref<Target = str>>(text: D) -> usize {
    text.len()
}

fn main() {
    let cell = RefCell::new(vec![1, 2, 3]);
    println!("{}", total(cell.borrow()));
    cell.borrow_mut().push(4);
    println!("{}", total(cell.borrow_mut()));
    println!("{}", total(Rc::new(vec![5, 6])));
    println!("{} {}", length(String::from("ada")), length(Rc::<str>::from("lovelace")));
    println!("{}", cell.try_borrow_mut().is_ok());
}
