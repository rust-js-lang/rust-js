//! A library's trait default, `then_all`, which holds what it's given's
//! iterator in the struct it gives back, and its free function, `paired`,
//! which makes both its arguments' iterators by their dictionaries, as
//! itertools' `interleave_shortest` and `zip_eq` do: a consumer gives each
//! as it is with its `IntoIterator` dictionary, a unit struct's
//! `undefined` too (itertools' laziness tests).
pub struct Both<I, J> {
    a: I,
    b: J,
}

impl<I: Iterator<Item = u8>, J: Iterator<Item = u8>> Iterator for Both<I, J> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.a.next().or_else(|| self.b.next())
    }
}

pub fn paired<I: IntoIterator, J: IntoIterator>(i: I, j: J) -> Both<I::IntoIter, J::IntoIter> {
    Both {
        a: i.into_iter(),
        b: j.into_iter(),
    }
}

pub trait Tools: Iterator {
    fn then_all<J: IntoIterator<Item = Self::Item>>(self, other: J) -> Both<Self, J::IntoIter>
    where
        Self: Sized,
    {
        Both {
            a: self,
            b: other.into_iter(),
        }
    }

    fn firsts(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.take(2).collect()
    }
}

impl<T: Iterator> Tools for T {}
