//! An impl's own method whose where clause bounds `Self::Item`: called
//! through the impl's dictionary, which the library exports, its caller
//! gives that bound's dictionary after its arguments, as a call through any
//! dictionary does, and the impl is given the drops of its items' iterators
//! (#1080).

pub struct D(pub u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Held<I> {
    pub iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

/// Makes each item, holding nothing to drop itself.
pub struct Countdown(pub u64);

impl Iterator for Countdown {
    type Item = Option<D>;

    fn next(&mut self) -> Option<Option<D>> {
        self.0 -= 1;
        (self.0 > 4).then(|| (self.0 % 2 == 0).then(|| D(self.0)))
    }
}

pub struct Wrapper<I>(pub I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

pub trait Keeping: Iterator {
    fn held(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator;
}

impl<I: Iterator> Keeping for Wrapper<I> {
    fn held(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let all: Vec<_> = self.map(|i| Held { iter: i.into_iter() }).collect();
        all.len()
    }
}
