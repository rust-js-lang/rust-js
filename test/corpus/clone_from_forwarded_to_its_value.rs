// `clone_from` that std forwards to the value's own: a `Box`'s
// `(**self).clone_from(&**source)`, a `Reverse`'s `self.0.clone_from(..)`,
// an `Option`'s of two `Some`s, an `Option` of an `Option`, of a `Box` and of
// a `Result`, each its value's, the crate's own or one std writes in place,
// and otherwise `*dest = source.clone()`, what it held dropped (#927). And
// a generic local given its caller's `clone_from` or none, which writes the
// clone where it's none, never the place's old value back.
use std::cmp::Reverse;
use std::collections::VecDeque;

#[derive(Debug)]
struct C(u32);

impl Clone for C {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        C(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

#[derive(Debug, Clone)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct P(u32);

fn assign_local<T: Clone + std::fmt::Debug>(start: T, source: &T) -> T {
    let mut local = start;
    local.clone_from(source);
    local
}

fn main() {
    let mut b = Box::new(C(4));
    b.clone_from(&Box::new(C(3)));
    println!("{b:?}");
    let mut rv = Reverse(C(6));
    rv.clone_from(&Reverse(C(5)));
    println!("{:?}", rv.0);
    let mut o: Option<Option<C>> = Some(Some(C(8)));
    o.clone_from(&Some(Some(C(7))));
    println!("{o:?}");
    o.clone_from(&Some(None));
    println!("{o:?}");
    o.clone_from(&None);
    println!("{o:?}");
    let mut q = Reverse(VecDeque::from([D(4)]));
    q.clone_from(&Reverse(VecDeque::from([D(5), D(6)])));
    println!("{:?}", q.0);
    let mut or: Option<Result<C, u8>> = Some(Ok(C(14)));
    or.clone_from(&Some(Ok(C(13))));
    println!("{or:?}");
    or.clone_from(&Some(Err(2)));
    println!("{or:?}");
    let mut ob: Option<Box<C>> = Some(Box::new(C(12)));
    ob.clone_from(&Some(Box::new(C(11))));
    println!("{ob:?}");
    ob.clone_from(&None);
    println!("{ob:?}");
    println!("{:?}", assign_local(P(1), &P(2)));
    println!("{:?}", assign_local(C(21), &C(22)));
    println!("{:?}", assign_local(Some(P(3)), &Some(P(4))));
    println!("{:?}", assign_local(None, &Some(C(23))));
    println!("{:?}", assign_local(Box::new(C(24)), &Box::new(C(25))));
}
