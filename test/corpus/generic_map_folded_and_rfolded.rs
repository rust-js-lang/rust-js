//@ library-differs
// A wrapper of a generic iterator, as the `either` crate's `IterEither`
// is: its `fold` and its `rfold` each map the iterator it holds, one map
// type stepped from its back by one impl and folded forward by the other,
// whose iterator is only an `Iterator`. The forward fold needs no back.

#[derive(Debug)]
struct Tagged<T>(T);

struct Wrap<I> {
    inner: I,
}

impl<I: Iterator> Iterator for Wrap<I> {
    type Item = Tagged<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Tagged)
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.inner.map(Tagged).fold(init, f)
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Wrap<I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(Tagged)
    }

    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.inner.map(Tagged).rfold(init, f)
    }
}

fn kept<T>(_: &T) -> bool {
    true
}

// And a filter of it, counted forward by one and stepped from its back by
// the other.
impl<I: Iterator> Wrap<I> {
    fn count_kept(self) -> usize {
        self.inner.filter(kept).count()
    }
}

impl<I: DoubleEndedIterator> Wrap<I> {
    fn last_kept(self) -> Option<I::Item> {
        self.inner.filter(kept).next_back()
    }
}

fn main() {
    let forward = Wrap { inner: 1..4 }.fold(Vec::new(), |mut all, Tagged(n)| {
        all.push(n);
        all
    });
    let backward = Wrap { inner: 1..4 }.rfold(Vec::new(), |mut all, Tagged(n)| {
        all.push(n);
        all
    });
    // One that steps forward only, which has no back to give.
    let words = Wrap { inner: std::iter::repeat("a").take(3) }.fold(String::new(), |all, Tagged(w)| all + w);
    println!("{forward:?} {backward:?} {words}");
    let mut each = Wrap { inner: 4..6 };
    println!("{:?} {:?} {:?}", each.next_back(), each.next(), each.next());
    let counted = Wrap { inner: std::iter::repeat("b").take(2) }.count_kept();
    println!("{counted} {:?}", Wrap { inner: 6..8 }.last_kept());
}
