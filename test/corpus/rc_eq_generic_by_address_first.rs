//@ library-differs
// `==` of two `Rc`s, or `Arc`s, in generic code bounded only by
// `T: PartialEq` asks whether they're one allocation first where the
// caller's `T` is `Eq`, as std's `RcEqIdent` specializes each instance, and
// calls `eq` always where it's only `PartialEq` (#931): the caller says
// which, `TEq`.
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug)]
struct D(u32);

impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        println!("eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

impl Eq for D {}

/// Only `PartialEq`, whose `==` std always calls.
#[derive(Debug)]
struct P(u32);

impl PartialEq for P {
    fn eq(&self, other: &P) -> bool {
        println!("partial eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

fn same<T: PartialEq>(a: &Rc<T>, b: &Rc<T>) -> bool {
    a == b
}

fn differ<T: PartialEq>(a: &Arc<T>, b: &Arc<T>) -> bool {
    a != b
}

/// Through a caller of its own, whose `T` is its caller's too.
fn both<T: PartialEq>(a: &Rc<T>, b: &Rc<T>) -> (bool, bool) {
    (same(a, b), a == &a.clone())
}

/// Of items of a `Vec`, and in a closure.
fn found<T: PartialEq>(items: &[Rc<T>], item: &Rc<T>) -> (bool, usize) {
    (items.contains(item), items.iter().filter(|each| *each == item).count())
}

/// Of a `Vec<T>`, `Eq` where `T` is.
fn same_items<T: PartialEq>(a: &Rc<Vec<T>>, b: &Rc<Vec<T>>) -> bool {
    a == b
}

struct Holder<T>(Rc<T>);

impl<T: PartialEq> PartialEq for Holder<T> {
    fn eq(&self, other: &Holder<T>) -> bool {
        self.0 == other.0
    }
}

fn main() {
    let a = Rc::new(D(1));
    let c = Rc::new(D(1));
    println!("{} {}", same(&a, &a.clone()), same(&a, &c));
    let x = Arc::new(D(2));
    println!("{} {}", differ(&x, &x.clone()), differ(&x, &Arc::new(D(3))));
    println!("{:?}", both(&a, &c));
    println!("{:?}", found(&[a.clone(), c.clone()], &a));
    println!("{}", Holder(a.clone()) == Holder(a.clone()));
    let p = Rc::new(P(3));
    println!("{} {:?}", same(&p, &p.clone()), both(&p, &p));
    println!("{:?}", found(&[p.clone()], &p));
    println!("{}", Holder(p.clone()) == Holder(p.clone()));
    let v = Rc::new(vec![D(6)]);
    let w = Rc::new(vec![P(7)]);
    println!("{} {}", same_items(&v, &v.clone()), same_items(&w, &w.clone()));
    let n = Rc::new(5);
    println!("{} {}", same(&n, &n.clone()), same(&n, &Rc::new(5)));
    let f = Rc::new(f64::NAN);
    println!("{} {}", same(&f, &f.clone()), differ(&Arc::new(1.5), &Arc::new(1.5)));
}
