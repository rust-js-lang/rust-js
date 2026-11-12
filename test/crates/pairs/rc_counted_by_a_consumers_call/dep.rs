//! A library's generic functions of an `Rc<T>`, none of which reads a count
//! or is called by its own code: its JS treats an `Rc` of `T` as not
//! counted. A consumer whose call gives one it counts, a `u32` whose count
//! it reads, compiles its own of each such function it gives one to: `get`,
//! `twice`, and `make_later`, whose closure alone holds one (#1234).
use std::rc::Rc;

pub fn get<T: Copy>(r: &Rc<T>) -> T {
    **r
}

pub fn twice<T>(r: &Rc<T>) -> (Rc<T>, Rc<T>) {
    (Rc::clone(r), Rc::clone(r))
}

pub fn make_later<T: Copy>(value: T) -> impl Fn() -> T {
    move || {
        let r = Rc::new(value);
        let s = Rc::clone(&r);
        *s
    }
}
