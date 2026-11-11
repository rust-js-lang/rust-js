//! A library's blanket impl a consumer compiles itself (ADR 0914), as
//! itertools' `impl<T: Iterator + ?Sized> Itertools for T` is in test_core:
//! a default it keeps, `last_of`, collects what it's given into a `Vec`,
//! which asks the size of `<Self as Iterator>::Item`, a fact of `T` the
//! impl's dictionary is given of its callers, as a function's is (#1409).
pub trait Tools: Iterator {
    fn next_array<const N: usize>(&mut self) -> Option<[Self::Item; N]>
    where
        Self: Sized,
    {
        next_array(self)
    }

    fn collect_array<const N: usize>(mut self) -> Option<[Self::Item; N]>
    where
        Self: Sized,
    {
        self.next_array().filter(|_| self.next().is_none())
    }

    /// Its last `n`, from its back, collected, as a `Vec` folds what's
    /// `TrustedLen` and steps what isn't (#670).
    fn last_of(self, n: usize) -> Vec<Self::Item>
    where
        Self: DoubleEndedIterator + Sized,
    {
        self.rev().skip(n).collect()
    }

    /// Its sum of products with `other`'s, a zip whose fold asks whether
    /// each side is `TrustedLen` (ADR 0975).
    fn dot(self, other: Vec<i32>) -> i32
    where
        Self: Iterator<Item = i32> + Sized,
    {
        self.zip(other).map(|(a, b)| a * b).sum()
    }
}

impl<T: Iterator + ?Sized> Tools for T {}

fn next_array<I: Iterator, const N: usize>(it: &mut I) -> Option<[I::Item; N]> {
    let mut items: [Option<I::Item>; N] = [(); N].map(|_| None);
    for slot in &mut items {
        *slot = Some(it.next()?);
    }
    Some(items.map(Option::unwrap))
}

pub fn partition<'a, A: 'a, I, F>(iter: I, mut pred: F) -> usize
where
    I: IntoIterator<Item = &'a mut A>,
    I::IntoIter: DoubleEndedIterator,
    F: FnMut(&A) -> bool,
{
    let mut split_index = 0;
    let mut iter = iter.into_iter();
    while let Some(front) = iter.next() {
        if !pred(front) {
            match iter.rfind(|back| pred(back)) {
                Some(back) => std::mem::swap(front, back),
                None => break,
            }
        }
        split_index += 1;
    }
    split_index
}
