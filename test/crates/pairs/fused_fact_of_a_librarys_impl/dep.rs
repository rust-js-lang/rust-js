// A library's iterator holding a `Fuse` of its own type parameter, whose
// `rfold` steps it from its back through std's adapters,
// `a.by_ref().rev()`, as itertools' `ZipLongest::rfold` does: its consumer
// compiles it, and gives it whether that parameter is a `FusedIterator`
// (#2042).
use std::iter::Fuse;

pub struct Pair<T> {
    a: Fuse<T>,
}

pub fn pair<T: Iterator>(a: T) -> Pair<T> {
    Pair { a: a.fuse() }
}

impl<T: DoubleEndedIterator + ExactSizeIterator> Iterator for Pair<T> {
    type Item = T::Item;
    fn next(&mut self) -> Option<T::Item> {
        self.a.next()
    }
}

impl<T: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Pair<T> {
    fn next_back(&mut self) -> Option<T::Item> {
        self.a.next_back()
    }

    fn rfold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let Self { mut a } = self;
        let n = a.len() / 2;
        let init = a.by_ref().rev().take(n).fold(init, &mut f);
        let init = a.by_ref().rev().fold(init, &mut f);
        println!("drained");
        let again = a.by_ref().rev().next();
        println!("again {}", again.is_some());
        init
    }
}
