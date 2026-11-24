//! A library's trait default that makes each item of its iterator into an
//! iterator of its own, `IntoIterator::into_iter`, and holds them in a
//! struct whose fields only its own code reads, as itertools'
//! `multi_cartesian_product` holds its `MultiProduct`. Its consumer gives it
//! items JS holds as no object, a unit struct's `undefined` and a fieldless
//! enum's string, each held by its generic code in a place of its own
//! (#2466).
pub struct Product<I> {
    iters: Vec<I>,
}

impl<I: Iterator> Product<I> {
    pub fn hints(&self) -> Vec<(usize, Option<usize>)> {
        self.iters.iter().map(|i| i.size_hint()).collect()
    }

    pub fn firsts(&mut self) -> Vec<Option<I::Item>> {
        self.iters.iter_mut().map(|i| i.next()).collect()
    }
}

pub trait Tools: Iterator {
    fn product_of(self) -> Product<<Self::Item as IntoIterator>::IntoIter>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        Product {
            iters: self.map(|i| i.into_iter()).collect(),
        }
    }
}

impl<T: Iterator> Tools for T {}
