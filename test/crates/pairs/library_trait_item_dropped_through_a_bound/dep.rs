//! A library's trait whose items a consumer's values may drop, a named
//! associated type's and a method's `impl Trait`, given to the library's
//! generic code that drops them where it gets them back through a bound of
//! its own, `relay_same<U>(x: impl Same<U>)` (#2007).
pub struct Loud(pub &'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub trait Source {
    type Named;
    fn named(&self, tag: &'static str) -> Self::Named;
    fn hidden(&self, tag: &'static str) -> impl Sized;
}

pub struct Louds;

impl Source for Louds {
    type Named = Loud;

    fn named(&self, tag: &'static str) -> Loud {
        Loud(tag)
    }

    fn hidden(&self, tag: &'static str) -> impl Sized {
        Loud(tag)
    }
}

pub fn at_2<T>(_x: T) {}

pub trait Same<X> {
    fn same(self) -> X;
}

impl<X> Same<X> for X {
    fn same(self) -> X {
        self
    }
}

pub fn relay_same<U>(x: impl Same<U>) {
    at_2(x.same());
}

pub fn paths<T: Source>(t: &T) {
    relay_same(t.named("named relayed by a bound"));
    relay_same(t.hidden("hidden relayed by a bound"));
    at_2(t.named("named given"));
    println!("end");
}
