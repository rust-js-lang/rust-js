//@ library-differs
// A generic impl whose own bound is a trait with a method that takes a
// `&mut Self` within what it takes, `Option<&mut Self>`, calling it through
// its type parameter's dictionary: of a number's impl, whose box generic
// code's is, its entry takes it as it is; of an object's, `Clicks`'s, its
// entry makes generic code's handle the object, `other?.value` (#685,
// #686). Given to generic code, called where its type is known and made a
// `dyn`.

#[derive(Debug)]
pub struct Clicks {
    n: u32,
}

impl Pick for Clicks {
    fn pick(&mut self, other: Option<&mut Self>) {
        if let Some(o) = other {
            o.n += 50;
        }
        self.n += 1;
    }
}

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
    let mut h = Holder { a: Clicks { n: 0 }, b: Clicks { n: 1 } };
    go(&mut h);
    println!("{h:?}");
    h.wrap();
    println!("{h:?}");
    let d: &mut dyn Wrap = &mut h;
    d.wrap();
    println!("{h:?}");
}
