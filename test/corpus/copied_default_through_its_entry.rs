//@ library-differs
// A trait's default method, copied into each impl, calls its trait's other
// methods on `Self`: where the impl's entry makes a `&mut` the impl's own,
// the object of a struct's `&mut Self`, it calls through the entry,
// `clicksTouch.me(self)`, as generic code does, so a `&mut Self` it's
// given back is generic code's box and one it gives within an `Option`
// the impl's object (#690). Of a struct's impl, a number's, and a generic
// impl's, called where its type is known and through generic code; and
// one of no `&mut self`, whose entry adapts only what it gives back.

#[derive(Debug)]
pub struct Clicks {
    n: u32,
}

pub trait Touch: Sized {
    fn me(&mut self) -> &mut Self;
    fn maybe(&mut self, mine: bool) -> Option<&mut Self>;
    fn touch(&mut self, other: &mut Self);
    fn touch_some(&mut self, other: Option<&mut Self>);
    fn fresh() -> Self;
    // Of no `&mut self`, whose entry adapts only what it gives back.
    fn last_of(items: &mut Vec<Self>) -> &mut Self;
    fn first_of(items: &mut Vec<Self>) -> Option<&mut Self>;

    // What those give back, written whole.
    fn renew_ends(items: &mut Vec<Self>) {
        *Self::last_of(items) = Self::fresh();
        if let Some(r) = Self::first_of(items) {
            *r = Self::fresh();
        }
    }

    // A `&mut Self` given back, written whole and through.
    fn via_me(&mut self) {
        let r = self.me();
        *r = Self::fresh();
        r.touch_some(None);
    }

    // One given back within an `Option`.
    fn via_maybe(&mut self) -> bool {
        match self.maybe(true) {
            Some(r) => {
                *r = Self::fresh();
                true
            }
            None => false,
        }
    }

    // One given within an `Option`, a reborrow and then itself.
    fn twice_some(&mut self, other: &mut Self) {
        self.touch_some(Some(&mut *other));
        let r = &mut *other;
        self.touch_some(Some(r));
        self.touch_some(None);
    }

    // And as its own `&mut`, as before.
    fn twice(&mut self, other: &mut Self) {
        self.touch(&mut *other);
        self.touch(other);
    }
}

impl Touch for Clicks {
    fn me(&mut self) -> &mut Self {
        self
    }
    fn maybe(&mut self, mine: bool) -> Option<&mut Self> {
        mine.then_some(self)
    }
    fn touch(&mut self, other: &mut Self) {
        other.n += 50;
    }
    fn touch_some(&mut self, other: Option<&mut Self>) {
        self.n += 1;
        if let Some(o) = other {
            o.n += 7;
        }
    }
    fn fresh() -> Self {
        Clicks { n: 99 }
    }
    fn last_of(items: &mut Vec<Self>) -> &mut Self {
        items.last_mut().unwrap()
    }
    fn first_of(items: &mut Vec<Self>) -> Option<&mut Self> {
        items.first_mut()
    }
}

impl Touch for u32 {
    fn me(&mut self) -> &mut Self {
        self
    }
    fn maybe(&mut self, mine: bool) -> Option<&mut Self> {
        mine.then_some(self)
    }
    fn touch(&mut self, other: &mut Self) {
        *other += 50;
    }
    fn touch_some(&mut self, other: Option<&mut Self>) {
        *self += 1;
        if let Some(o) = other {
            *o += 7;
        }
    }
    fn fresh() -> Self {
        99
    }
    fn last_of(items: &mut Vec<Self>) -> &mut Self {
        items.last_mut().unwrap()
    }
    fn first_of(items: &mut Vec<Self>) -> Option<&mut Self> {
        items.first_mut()
    }
}

#[derive(Debug)]
pub struct Holder<T> {
    t: T,
}

impl<T: Touch> Touch for Holder<T> {
    fn me(&mut self) -> &mut Self {
        self
    }
    fn maybe(&mut self, mine: bool) -> Option<&mut Self> {
        mine.then_some(self)
    }
    fn touch(&mut self, other: &mut Self) {
        self.t.touch(&mut other.t);
    }
    fn touch_some(&mut self, other: Option<&mut Self>) {
        self.t.touch_some(other.map(|o| &mut o.t));
    }
    fn fresh() -> Self {
        Holder { t: T::fresh() }
    }
    fn last_of(items: &mut Vec<Self>) -> &mut Self {
        items.last_mut().unwrap()
    }
    fn first_of(items: &mut Vec<Self>) -> Option<&mut Self> {
        items.first_mut()
    }
}

fn all<T: Touch + std::fmt::Debug>(a: &mut T, b: &mut T) -> String {
    a.via_me();
    let maybe = a.via_maybe();
    a.twice_some(b);
    a.twice(b);
    format!("{maybe} {a:?} {b:?}")
}

fn renewed<T: Touch + std::fmt::Debug>(mut items: Vec<T>) -> String {
    T::renew_ends(&mut items);
    format!("{items:?}")
}

fn main() {
    let (mut a, mut b) = (Clicks { n: 1 }, Clicks { n: 2 });
    a.via_me();
    println!("{a:?}");
    let maybe = a.via_maybe();
    println!("{maybe} {a:?}");
    a.twice_some(&mut b);
    a.twice(&mut b);
    println!("{a:?} {b:?}");
    println!("{}", all(&mut a, &mut b));
    let (mut x, mut y) = (1u32, 2u32);
    x.via_me();
    x.twice_some(&mut y);
    println!("{x} {y}");
    println!("{}", all(&mut x, &mut y));
    let (mut h, mut k) = (Holder { t: Clicks { n: 1 } }, Holder { t: Clicks { n: 2 } });
    h.via_me();
    h.twice_some(&mut k);
    println!("{h:?} {k:?}");
    println!("{}", all(&mut h, &mut k));
    let mut items = vec![Clicks { n: 1 }, Clicks { n: 2 }, Clicks { n: 3 }];
    Clicks::renew_ends(&mut items);
    println!("{items:?}");
    println!("{}", renewed(vec![Clicks { n: 1 }, Clicks { n: 2 }]));
    println!("{}", renewed(vec![1u32, 2, 3]));
    println!("{}", renewed(vec![Holder { t: Clicks { n: 1 } }, Holder { t: Clicks { n: 2 } }]));
}
