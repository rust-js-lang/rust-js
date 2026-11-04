//@ library-differs
// A generic impl whose own bound is a trait with a method that takes a
// `&mut Self` within what it takes, `Option<&mut Self>`, calling it through
// its type parameter's dictionary of a number's impl: generic code's box is
// the number's own, so its entry takes it as it is. Given to generic code,
// called where its type is known and made a `dyn`. Of an object's impl, its
// dictionary is refused where it's made (#685).

pub trait Pick {
    fn pick(&mut self, other: Option<&mut Self>);
}

impl Pick for u32 {
    fn pick(&mut self, other: Option<&mut Self>) {
        if let Some(o) = other {
            *o += 50;
        }
        *self += 1;
    }
}

pub trait Wrap {
    fn wrap(&mut self);
}

#[derive(Debug)]
pub struct Holder<T> {
    a: T,
    b: T,
}

impl<T: Pick> Wrap for Holder<T> {
    fn wrap(&mut self) {
        self.a.pick(Some(&mut self.b));
        self.b.pick(None);
    }
}

fn go<W: Wrap>(w: &mut W) {
    w.wrap();
}

fn main() {
    let mut h = Holder { a: 0u32, b: 1 };
    go(&mut h);
    println!("{h:?}");
    h.wrap();
    println!("{h:?}");
    let d: &mut dyn Wrap = &mut h;
    d.wrap();
    println!("{h:?}");
}
