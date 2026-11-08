//! A generic iterator whose `next` gives an item to a closure, as
//! itertools' `Product` and `kmerge`'s `HeadTail` do: an `Option::map` of
//! `None` drops the closure, and so the item it took (#840).
pub struct Paired<I: Iterator> {
    pub it: I,
}

impl<I: Iterator> Iterator for Paired<I> {
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.it.next()?;
        self.it.next().map(|second| (first, second))
    }
}
