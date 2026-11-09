//! A generic impl whose method gives its type parameter's trait method a
//! `Vec<&mut T>` through `T`'s dictionary (#685, #686).
pub trait Pick {
    fn pick(&mut self, o: Vec<&mut Self>);
}

pub trait Wrap {
    fn wrap(&mut self);
}

pub struct H<T> {
    pub a: T,
    pub b: T,
}

impl<T: Pick> Wrap for H<T> {
    fn wrap(&mut self) {
        self.a.pick(vec![&mut self.b])
    }
}

pub fn go<W: Wrap>(w: &mut W) {
    w.wrap()
}
