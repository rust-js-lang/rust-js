//! A library's trait whose generic method counts the `Rc` it's given, and
//! the library's generic code that makes one and gives it.
use std::rc::Rc;

pub trait Keep {
    fn keep<T>(&self, r: &Rc<T>) -> usize {
        let again = Rc::clone(r);
        Rc::strong_count(&again)
    }
}

pub struct A;
impl Keep for A {}

pub struct B;
impl Keep for B {
    fn keep<T>(&self, r: &Rc<T>) -> usize {
        Rc::strong_count(r) * 10
    }
}

pub fn by_default<K: Keep>(k: &K) -> usize {
    let r = Rc::new(vec![1u8]);
    let _two = r.clone();
    k.keep(&r)
}
