//! A library's extension trait every iterator has, by a blanket impl, as
//! itertools' `impl<T: Iterator + ?Sized> Itertools for T` is, whose default
//! of a const generic hands `&mut self` to another default that steps it, as
//! `collect_array` hands it to `next_array`; and a free function of what an
//! `IntoIterator` makes stepped from both ends, as `partition` is, which a
//! consumer compiles itself where it's given a `&mut` of an array (ADR 0914,
//! #1409).
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
