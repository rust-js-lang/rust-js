//! A library's struct of iterators, its field private as itertools'
//! `MultiProduct`'s is, filled with what an `IntoIterator` bound's
//! `into_iter()` makes: a free function's, an inherent method's own and a
//! trait default's own (#2508, #2520).
pub struct Held<I> {
    iters: Vec<I>,
}

impl<I: Iterator> Held<I> {
    pub fn new(first: I) -> Self {
        Held { iters: vec![first] }
    }

    pub fn firsts(&mut self) -> Vec<Option<I::Item>> {
        self.iters.iter_mut().map(|i| i.next()).collect()
    }

    pub fn hints(&self) -> Vec<(usize, Option<usize>)> {
        self.iters.iter().map(|i| i.size_hint()).collect()
    }

    pub fn with<J: IntoIterator<IntoIter = I>>(mut self, more: J) -> Self {
        self.iters.push(more.into_iter());
        self
    }
}

pub fn held_of<T>(items: T) -> Held<<T::Item as IntoIterator>::IntoIter>
where
    T: IntoIterator,
    T::Item: IntoIterator,
{
    Held {
        iters: items.into_iter().map(|i| i.into_iter()).collect(),
    }
}

pub trait Tools: Iterator {
    fn held_with<J>(self, first: J) -> Held<J::IntoIter>
    where
        Self: Sized + Iterator<Item = J>,
        J: IntoIterator,
    {
        let mut iters = vec![first.into_iter()];
        iters.extend(self.map(|i| i.into_iter()));
        Held { iters }
    }

    fn nested(self) -> Vec<Held<<Self::Item as IntoIterator>::IntoIter>>
    where
        Self: Sized,
        Self::Item: IntoIterator + Clone,
    {
        self.map(|i| Held {
            iters: vec![i.clone().into_iter(), i.into_iter()],
        })
        .collect()
    }
}

impl<T: Iterator> Tools for T {}
