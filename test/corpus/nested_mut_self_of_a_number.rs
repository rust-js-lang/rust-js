// A trait's method that takes a `&mut Self` within what it takes, an
// `Option` or a tuple, called by generic code through a dictionary of a
// number's impl: generic code's box is the number's own, so its entry takes
// it as it is. Of an object's impl, the call is refused (#682).

pub trait Pick {
    fn pick(&mut self, other: Option<&mut Self>) -> bool;
    fn both(&self, others: (&mut Self, u32)) -> u32;
}

impl Pick for u32 {
    fn pick(&mut self, other: Option<&mut Self>) -> bool {
        match other {
            Some(o) => {
                *o += *self;
                true
            }
            None => false,
        }
    }

    fn both(&self, others: (&mut Self, u32)) -> u32 {
        *others.0 += others.1 * *self;
        *others.0
    }
}

fn go<P: Pick>(p: &mut P, q: &mut P) -> (bool, bool, u32) {
    let picked = p.pick(Some(q));
    let none = p.pick(None);
    (picked, none, p.both((q, 3)))
}

fn main() {
    let mut a = 5u32;
    let mut b = 7u32;
    println!("{:?} {a} {b}", go(&mut a, &mut b));
}
