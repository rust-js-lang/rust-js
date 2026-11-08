//! A library's generic functions whose signatures hold what an
//! `IntoIterator` makes as it is, which take its dictionary: in a struct
//! of the library's, given back by an impl's sibling method, and hidden
//! behind an `impl Iterator`. Its consumer gives them as the library's own
//! compile decided, of the same signatures (#1025).
pub struct Both<I, J> {
    pub a: I,
    pub b: J,
}

pub fn both<I: IntoIterator, J: IntoIterator>(a: I, b: J) -> Both<I::IntoIter, J::IntoIter> {
    Both {
        a: a.into_iter(),
        b: b.into_iter(),
    }
}

pub struct Pair<T> {
    pub t: T,
}

pub struct Wrap<A>(pub A);

impl<A: IntoIterator> Wrap<A> {
    pub fn pair(self) -> Pair<A::IntoIter> {
        Pair { t: self.0.into_iter() }
    }

    pub fn first(self) -> Option<A::Item> {
        self.pair().t.next()
    }
}

pub fn chained<I: IntoIterator, J: IntoIterator<Item = I::Item>>(a: I, b: J) -> impl Iterator<Item = I::Item> {
    a.into_iter().chain(b)
}
