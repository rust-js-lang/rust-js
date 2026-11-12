//@ compile-fail: `std::cell::OnceCell::<T>::get` in a crate that asks an address (#1513) yet: it lends `std::option::Option<P>` from inside its `std::cell::OnceCell<std::option::Option<P>>`
// std's function that lends an object from its receiver's own storage, in a
// crate that asks an address of what it lends, where JS keeps the payload in
// a `$some` box, a `OnceCell` of an `Option` of a struct's: refused, naming
// #1513, as the slot holds the box, not the object, whose address would be
// its own, not inside the `OnceCell`'s. Of a payload held as the object
// itself, the call records it as stored in the slot (#1475).
use std::cell::OnceCell;

#[repr(C)]
#[derive(Debug)]
struct P {
    a: u8,
    b: u32,
}

fn main() {
    let once = OnceCell::new();
    once.set(Some(P { a: 1, b: 2 })).unwrap();
    let held = once.get().unwrap() as *const Option<P> as usize;
    println!("{}", held - &raw const once as usize);
    println!("{:?}", once.get());
}
