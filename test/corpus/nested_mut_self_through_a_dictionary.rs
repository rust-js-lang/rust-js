//@ library-differs
// A trait's method that takes or gives a `&mut Self` within an `Option` or
// a tuple, called by generic code through a dictionary: each impl's entry
// makes generic code's box of it its own form, the object of a struct's
// impl, `other?.value`, and its own of what it gives generic code's box,
// `$cellOf(..)`; a number's, whose box generic code's is, takes it as it
// is (#686). Of an object's impl, a number's, a generic impl's and a
// generic impl's own bound. (A default method's call of one is #690's.)

#[derive(Debug, Clone, PartialEq)]
pub struct Clicks {
    n: u32,
}

pub trait Pick {
    fn pick(&mut self, other: Option<&mut Self>) -> bool;
    fn both(&self, others: (&mut Self, u32)) -> u32;
    fn deeper(&mut self, others: Option<(&mut Self, &mut Self)>);
    fn first<'a>(&'a mut self, other: &'a mut Self, mine: bool) -> Option<&'a mut Self>;
    fn pair<'a>(&'a mut self, other: &'a mut Self) -> (&'a mut Self, u32);
}

impl Pick for Clicks {
    fn pick(&mut self, other: Option<&mut Self>) -> bool {
        self.n += 1;
        match other {
            Some(o) => {
                o.n += 50;
                true
            }
            None => false,
        }
    }

    fn both(&self, others: (&mut Self, u32)) -> u32 {
        others.0.n += others.1 * self.n;
        others.0.n
    }

    fn deeper(&mut self, others: Option<(&mut Self, &mut Self)>) {
        if let Some((a, b)) = others {
            a.n += 1000;
            *b = Clicks { n: b.n + self.n };
        }
    }

    fn first<'a>(&'a mut self, other: &'a mut Self, mine: bool) -> Option<&'a mut Self> {
        match mine {
            true => Some(self),
            false if other.n > 0 => Some(other),
            false => None,
        }
    }

    fn pair<'a>(&'a mut self, other: &'a mut Self) -> (&'a mut Self, u32) {
        let n = other.n;
        (self, n)
    }
}

impl Pick for u32 {
    fn pick(&mut self, other: Option<&mut Self>) -> bool {
        *self += 1;
        match other {
            Some(o) => {
                *o += 50;
                true
            }
            None => false,
        }
    }

    fn both(&self, others: (&mut Self, u32)) -> u32 {
        *others.0 += others.1 * *self;
        *others.0
    }

    fn deeper(&mut self, others: Option<(&mut Self, &mut Self)>) {
        if let Some((a, b)) = others {
            *a += 1000;
            *b += *self;
        }
    }

    fn first<'a>(&'a mut self, other: &'a mut Self, mine: bool) -> Option<&'a mut Self> {
        match mine {
            true => Some(self),
            false if *other > 0 => Some(other),
            false => None,
        }
    }

    fn pair<'a>(&'a mut self, other: &'a mut Self) -> (&'a mut Self, u32) {
        let n = *other;
        (self, n)
    }
}

// A generic impl, of an object, whose `Self` holds its `T`.
#[derive(Debug, Clone)]
pub struct Holder<T> {
    t: T,
}

impl<T: Pick> Pick for Holder<T> {
    fn pick(&mut self, other: Option<&mut Self>) -> bool {
        self.t.pick(other.map(|o| &mut o.t))
    }

    fn both(&self, others: (&mut Self, u32)) -> u32 {
        self.t.both((&mut others.0.t, others.1))
    }

    fn deeper(&mut self, others: Option<(&mut Self, &mut Self)>) {
        self.t.deeper(others.map(|(a, b)| (&mut a.t, &mut b.t)));
    }

    fn first<'a>(&'a mut self, other: &'a mut Self, mine: bool) -> Option<&'a mut Self> {
        match mine {
            true => Some(self),
            false => Some(other),
        }
    }

    fn pair<'a>(&'a mut self, other: &'a mut Self) -> (&'a mut Self, u32) {
        let n = self.t.both((&mut other.t, 0));
        (self, n)
    }
}

// A reborrow given within, then the `&mut` itself.
fn pick_twice<P: Pick>(p: &mut P, other: &mut P) {
    p.pick(Some(&mut *other));
    p.pick(Some(other));
}

fn go<P: Pick + Clone + std::fmt::Debug>(p: &mut P, q: &mut P) -> String {
    let picked = p.pick(Some(q));
    let none = p.pick(None);
    let both = p.both((q, 3));
    let mut extra = q.clone();
    p.deeper(Some((&mut extra, q)));
    p.deeper(None);
    pick_twice(p, q);
    let mut out = format!("{picked} {none} {both} {p:?} {q:?} {extra:?}");
    let mut other = q.clone();
    if let Some(r) = p.first(&mut other, true) {
        r.pick(None);
    }
    if let Some(r) = p.first(&mut other, false) {
        r.pick(None);
    }
    let (r, n) = p.pair(&mut other);
    r.pick(None);
    out += &format!(" {n} {p:?} {other:?}");
    out
}

// Called by JS as declared, given each impl's dictionary
// (test/declarations-conformance.test.ts).
pub fn picked<P: Pick>(p: &mut P, q: &mut P) -> bool {
    p.pick(Some(q)) && !p.pick(None)
}

pub fn both_of<P: Pick>(p: &P, q: &mut P) -> u32 {
    p.both((q, 3))
}

pub fn first_of<P: Pick>(p: &mut P, q: &mut P, mine: bool) -> bool {
    match p.first(q, mine) {
        Some(r) => r.pick(None),
        None => false,
    }
}

fn main() {
    let (mut a, mut b) = (Clicks { n: 1 }, Clicks { n: 2 });
    println!("{}", go(&mut a, &mut b));
    println!("{a:?} {b:?}");
    let (mut x, mut y) = (5u32, 7u32);
    println!("{}", go(&mut x, &mut y));
    println!("{x} {y}");
    let (mut h, mut k) = (Holder { t: Clicks { n: 3 } }, Holder { t: Clicks { n: 4 } });
    println!("{}", go(&mut h, &mut k));
    println!("{h:?} {k:?}");
    let (mut h, mut k) = (Holder { t: 3u32 }, Holder { t: 4u32 });
    println!("{}", go(&mut h, &mut k));
    println!("{h:?} {k:?}");
    let (mut c, mut d) = (Clicks { n: 1 }, Clicks { n: 0 });
    println!("{} {} {} {c:?} {d:?}", picked(&mut c, &mut d), both_of(&c, &mut d), first_of(&mut c, &mut d, false));
}
