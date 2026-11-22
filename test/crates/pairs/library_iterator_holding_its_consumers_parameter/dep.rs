//! A library's iterators in two modules, as itertools' `Combinations` and
//! `Powerset` are: `Powerset<I>`'s impl, which its consumer compiles itself
//! (ADR 0914), steps the `Combinations<I>` it holds, whose impl the library
//! exports, by that impl's `next` and `nth` and by std's `fold` of a `&mut`
//! to it, each stepping its consumer's `I` (#1420).
pub mod combinations {
    pub struct Combinations<I: Iterator> {
        indices: Vec<usize>,
        pool: Vec<I::Item>,
        it: I,
        first: bool,
    }

    pub fn combinations<I: Iterator>(it: I, k: usize) -> Combinations<I> {
        Combinations {
            indices: (0..k).collect(),
            pool: Vec::new(),
            it,
            first: true,
        }
    }

    impl<I: Iterator> Combinations<I> {
        pub fn k(&self) -> usize {
            self.indices.len()
        }

        pub fn n(&self) -> usize {
            self.pool.len()
        }

        fn prefill(&mut self, len: usize) {
            while self.pool.len() < len {
                match self.it.next() {
                    Some(x) => self.pool.push(x),
                    None => break,
                }
            }
        }

        pub fn reset(&mut self, k: usize) {
            self.first = true;
            self.indices = (0..k).collect();
        }

        fn increment_indices(&mut self) -> bool {
            if self.indices.is_empty() {
                return true;
            }
            let mut i = self.indices.len() - 1;
            if self.indices[i] == self.pool.len() - 1
                && let Some(x) = self.it.next()
            {
                self.pool.push(x);
            }
            while self.indices[i] == i + self.pool.len() - self.indices.len() {
                if i == 0 {
                    return true;
                }
                i -= 1;
            }
            self.indices[i] += 1;
            for j in i + 1..self.indices.len() {
                self.indices[j] = self.indices[j - 1] + 1;
            }
            false
        }
    }

    impl<I: Iterator> Iterator for Combinations<I>
    where
        I::Item: Clone,
    {
        type Item = Vec<I::Item>;

        fn next(&mut self) -> Option<Vec<I::Item>> {
            let done = if self.first {
                self.prefill(self.k());
                self.first = false;
                self.k() > self.n()
            } else {
                self.increment_indices()
            };
            if done {
                return None;
            }
            Some(self.indices.iter().map(|&i| self.pool[i].clone()).collect())
        }

        fn nth(&mut self, n: usize) -> Option<Vec<I::Item>> {
            for _ in 0..n {
                self.next()?;
            }
            self.next()
        }
    }
}

pub mod powerset {
    use crate::combinations::{Combinations, combinations};

    pub struct Powerset<I: Iterator> {
        combs: Combinations<I>,
    }

    pub fn powerset<I: Iterator>(it: I) -> Powerset<I> {
        Powerset {
            combs: combinations(it, 0),
        }
    }

    impl<I: Iterator> Powerset<I> {
        fn increment_k(&mut self) -> bool {
            if self.combs.k() < self.combs.n() || self.combs.k() == 0 {
                self.combs.reset(self.combs.k() + 1);
                true
            } else {
                false
            }
        }
    }

    impl<I: Iterator> Iterator for Powerset<I>
    where
        I::Item: Clone,
    {
        type Item = Vec<I::Item>;

        fn next(&mut self) -> Option<Vec<I::Item>> {
            if let Some(elt) = self.combs.next() {
                Some(elt)
            } else if self.increment_k() {
                self.combs.next()
            } else {
                None
            }
        }

        fn nth(&mut self, n: usize) -> Option<Vec<I::Item>> {
            if n == 0 {
                return self.next();
            }
            match self.combs.nth(n - 1) {
                Some(_) => self.combs.next(),
                None => {
                    self.increment_k();
                    self.next()
                }
            }
        }

        fn fold<B, F>(self, mut init: B, mut f: F) -> B
        where
            F: FnMut(B, Vec<I::Item>) -> B,
        {
            let mut it = self.combs;
            if it.k() == 0 {
                init = it.by_ref().fold(init, &mut f);
                it.reset(1);
            }
            init = it.by_ref().fold(init, &mut f);
            for k in it.k() + 1..=it.n() {
                it.reset(k);
                init = it.by_ref().fold(init, &mut f);
            }
            init
        }
    }
}

pub trait Itertools: Iterator {
    fn powerset(self) -> powerset::Powerset<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        powerset::powerset(self)
    }
}

impl<T: Iterator> Itertools for T {}
