//! A library's trait whose generic methods copy, clone, wrap in a `Some`
//! and drop what they're given, and the library's generic code that makes
//! each and gives it, which a consumer may compile itself.
pub trait Use {
    fn copied<T: Copy + std::fmt::Debug>(&self, t: T) -> String {
        let (a, b) = (t, t);
        format!("{a:?} {b:?}")
    }
    fn cloned<T: Clone>(&self, t: &T) -> (T, T) {
        (t.clone(), t.clone())
    }
    fn wrapped<T>(&self, t: T) -> Option<T> {
        Some(t)
    }
    fn dropped<T>(&self, t: T) -> usize {
        drop(t);
        1
    }
}

pub struct A;
impl Use for A {}

#[derive(Clone, Copy, Debug)]
pub struct P {
    pub x: u32,
}

pub struct Loud(pub &'static str);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn by_caller<U: Use>(u: &U) -> String {
    let copies = u.copied(P { x: 1 });
    let (mut a, b) = u.cloned(&vec![1u8]);
    a.push(2);
    let some = u.wrapped(());
    let none_inside = u.wrapped(None::<u8>);
    let n = u.dropped(Loud("given"));
    format!("{copies} {a:?} {b:?} {some:?} {none_inside:?} {n}")
}
