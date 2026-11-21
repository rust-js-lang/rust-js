//! A library's holder of what it's given's iterator, `Chunks<I>`, whose
//! fields only its own generic code reads, inside a `RefCell` of a struct
//! of its own, as itertools' `IntoChunks<I>` holds a `GroupInner<.., I,
//! ..>`: its trait default, `chunks_of`, and its free function, `chunked`,
//! make one, and its methods step it, each holding the iterator as its
//! generic code holds it wherever, so a consumer gives each an iterator JS
//! holds as no object (#2255). And a tuple of two taken and given back by
//! value, `zipped`'s, and made into an iterator by a `From<(A, B)>` impl,
//! as itertools' `multizip` makes its `Zip<(A, B)>`, part by part; and a
//! tuple's own impl of the library's trait, called by its consumer, whose
//! JS function the library wrote holding each part in a place.
use std::cell::RefCell;

struct Inner<I> {
    iter: I,
    taken: usize,
}

pub struct Chunks<I> {
    inner: RefCell<Inner<I>>,
    size: usize,
}

impl<I: Iterator> Chunks<I> {
    pub fn next_chunk(&self) -> Vec<I::Item> {
        let mut inner = self.inner.borrow_mut();
        let mut chunk = Vec::new();
        while chunk.len() < self.size {
            match inner.iter.next() {
                Some(item) => {
                    chunk.push(item);
                    inner.taken += 1;
                }
                None => break,
            }
        }
        chunk
    }

    pub fn taken(&self) -> usize {
        self.inner.borrow().taken
    }
}

pub fn chunked<I: IntoIterator>(items: I, size: usize) -> Chunks<I::IntoIter> {
    Chunks {
        inner: RefCell::new(Inner {
            iter: items.into_iter(),
            taken: 0,
        }),
        size,
    }
}

pub trait Tools: Iterator {
    fn chunks_of(self, size: usize) -> Chunks<Self>
    where
        Self: Sized,
    {
        Chunks {
            inner: RefCell::new(Inner { iter: self, taken: 0 }),
            size,
        }
    }
}

impl<T: Iterator> Tools for T {}

pub fn zipped<A: Iterator, B: Iterator>((mut a, mut b): (A, B)) -> (Option<(A::Item, B::Item)>, (A, B)) {
    let first = a.next().zip(b.next());
    (first, (a, b))
}

pub struct Zip<T> {
    t: T,
}

impl<A: IntoIterator, B: IntoIterator> From<(A, B)> for Zip<(A::IntoIter, B::IntoIter)> {
    fn from((a, b): (A, B)) -> Self {
        Zip {
            t: (a.into_iter(), b.into_iter()),
        }
    }
}

impl<A: Iterator, B: Iterator> Iterator for Zip<(A, B)> {
    type Item = (A::Item, B::Item);

    fn next(&mut self) -> Option<Self::Item> {
        self.t.0.next().zip(self.t.1.next())
    }
}

pub fn multizip<T, U>(t: U) -> Zip<T>
where
    Zip<T>: From<U>,
{
    Zip::from(t)
}

pub trait Step {
    fn step(self) -> Self;
}

impl<A: Iterator, B: Iterator> Step for (A, B) {
    fn step(mut self) -> Self {
        self.0.next();
        self.1.next();
        self
    }
}
