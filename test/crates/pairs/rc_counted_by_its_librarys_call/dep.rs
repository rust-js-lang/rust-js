//! A library whose generic `count` reads an `Rc<T>`'s count, so its JS
//! counts an `Rc` of a type parameter, `peek`'s too, and whose own code
//! counts an `Rc<u32>`. A consumer whose call gives one it doesn't count,
//! an `i8`, compiles its own (#1234).
use std::rc::Rc;

pub fn count<T>(r: &Rc<T>) -> usize {
    Rc::strong_count(r)
}

pub fn peek<T: Copy>(r: &Rc<T>) -> T {
    **r
}

pub fn own_count() -> usize {
    let r = Rc::new(7u32);
    let s = Rc::clone(&r);
    peek(&s) as usize + count(&r)
}
