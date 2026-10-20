// A crate that takes another's `Rc<T>` into a trait object, a callback and
// another `Rc` (ADR 0523).
use std::fmt::Debug;
use std::rc::Rc;

pub trait Show {
    fn show(&self) -> String;
}

impl<T: Debug> Show for Rc<T> {
    fn show(&self) -> String {
        format!("{:?} x{}", self, Rc::strong_count(self))
    }
}

pub fn boxed<T: Debug + 'static>(rc: Rc<T>) -> Box<dyn Show> {
    Box::new(rc)
}

pub fn callback<T: 'static>(rc: Rc<T>) -> impl Fn() -> usize {
    move || Rc::strong_count(&rc)
}

pub fn nest<T>(rc: Rc<T>) -> Rc<Rc<T>> {
    Rc::new(rc)
}
