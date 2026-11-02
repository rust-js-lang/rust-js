//! An iterator of either of two, as either's `Either<L, R>` is (#475):
//! its `DoubleEndedIterator`'s `nth_back` and `rfold`, which std's `Rev`
//! routes through, take its sides' `DoubleEndedIterator` dictionaries.
pub enum Two<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Iterator for Two<L, R>
where
    L: Iterator,
    R: Iterator<Item = L::Item>,
{
    type Item = L::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Two::Left(inner) => inner.next(),
            Two::Right(inner) => inner.next(),
        }
    }

    fn fold<Acc, G>(self, init: Acc, f: G) -> Acc
    where
        G: FnMut(Acc, Self::Item) -> Acc,
    {
        match self {
            Two::Left(inner) => inner.fold(init, f),
            Two::Right(inner) => inner.fold(init, f),
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            Two::Left(inner) => inner.nth(n),
            Two::Right(inner) => inner.nth(n),
        }
    }
}

impl<L, R> DoubleEndedIterator for Two<L, R>
where
    L: DoubleEndedIterator,
    R: DoubleEndedIterator<Item = L::Item>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Two::Left(inner) => inner.next_back(),
            Two::Right(inner) => inner.next_back(),
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            Two::Left(inner) => inner.nth_back(n),
            Two::Right(inner) => inner.nth_back(n),
        }
    }

    fn rfold<Acc, G>(self, init: Acc, f: G) -> Acc
    where
        G: FnMut(Acc, Self::Item) -> Acc,
    {
        match self {
            Two::Left(inner) => inner.rfold(init, f),
            Two::Right(inner) => inner.rfold(init, f),
        }
    }
}
