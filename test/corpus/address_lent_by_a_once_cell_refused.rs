//@ compile-fail: `std::cell::OnceCell::<T>::get` in a crate that asks an address (#1475) yet: it lends `P` from inside its `std::cell::OnceCell<P>`
// std's function that lends a reference into its receiver's own storage,
// whose place there rust-js doesn't know, a `OnceCell`'s `get`, a
// `Result`'s `as_ref`, a `Peekable`'s `peek`, in a crate that asks an
// address of what it lends: refused, naming #1475, as JS keeps what it
// holds apart from it, and its address would be its own, not inside the
// `OnceCell`'s.
use std::cell::OnceCell;

#[repr(C)]
#[derive(Debug)]
struct P {
    a: u8,
    b: u32,
}

fn main() {
    let once = OnceCell::new();
    once.set(P { a: 1, b: 2 }).unwrap();
    let held = once.get().unwrap() as *const P as usize;
    println!("{}", held - &raw const once as usize);
    println!("{:?}", once.get());
}
