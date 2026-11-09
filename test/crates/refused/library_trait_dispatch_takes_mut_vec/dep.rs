//! A trait whose method takes a `Vec<&mut Self>`, and generic code that
//! gives it one through `S`'s dictionary (#682).
pub trait Pick {
    fn pick(&mut self, o: Vec<&mut Self>);
}

pub fn go<S: Pick>(s: &mut S, t: &mut S) {
    s.pick(vec![t])
}
