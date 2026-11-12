//@ compile-fail: `std::cell::OnceCell::<T>::get_mut` in a crate that asks an address (#1513) yet: it lends `P<T>` from inside its `std::cell::OnceCell<P<T>>`
// Generic code's `&mut` to an object std lends from inside its receiver, a
// `OnceCell`'s of a struct, in a crate that asks its address: refused,
// naming #1513, as JS keeps the object apart from the `OnceCell` and no
// handle on a place can say where it is. Only a `&mut` the lowering makes a
// handle on its place, a number's, is #1478's, the offsets its callers give.
use std::cell::OnceCell;

#[derive(Debug)]
struct P<T> {
    a: u8,
    b: T,
}

fn offset<T>(once: &mut OnceCell<P<T>>) -> usize {
    let at = once.get_mut().unwrap() as *mut P<T> as usize;
    at - &raw const *once as usize
}

fn main() {
    let mut once = OnceCell::new();
    once.set(P { a: 1, b: 2u32 }).unwrap();
    println!("{}", offset(&mut once));
    let mut other = OnceCell::new();
    other.set(P { a: 1, b: 2u64 }).unwrap();
    println!("{}", offset(&mut other));
    println!("{:?} {:?}", once.get(), other.get());
}
