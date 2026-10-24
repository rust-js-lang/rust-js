//! A library whose generic code drops its struct holding an `Rc` of a type
//! parameter's value, which none of its expressions name: its consumers'
//! type may have a destructor, which the last `Rc` runs, so it counts every
//! `Rc` (ADRs 0494, 0844).
use std::rc::Rc;

pub struct Holder<T> {
    pub rc: Rc<T>,
}

pub fn release<T>(holder: Holder<T>) {
    println!("releasing");
    let _ = holder;
}
