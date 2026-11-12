//! A library's generic function holding two `Rc`s of one type parameter's,
//! an `Rc<Slot<T>>` whose count it reads, so counted, and an `Rc<T>`, not:
//! one mark of its parameter can't say each, so a consumer that counts its
//! `T`'s `Rc` compiles its own (#1234).
use std::rc::Rc;

pub struct Slot<T> {
    pub value: T,
}

pub fn both<T: Copy>(value: T) -> (Rc<T>, usize) {
    let slot = Rc::new(Slot { value });
    let again = Rc::clone(&slot);
    let n = Rc::strong_count(&again);
    (Rc::new(slot.value), n)
}

pub fn read<T: Copy>(r: &Rc<T>) -> T {
    **r
}
