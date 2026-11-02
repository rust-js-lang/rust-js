//@ library-differs
// The crate's iterator of either of two, as either's `Either<L, R>` is,
// whose own `fold`, `nth`, `rfold` and `nth_back`, which std's consumers
// and `Rev` route through, take its sides' dictionaries: each a function of
// what the runtime gives that gives it those of the type stepped (#475).

enum Two<L, R> {
    Left(L),
    Right(R),
}

impl<L: Iterator, R: Iterator<Item = L::Item>> Iterator for Two<L, R> {
    type Item = L::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Two::Left(inner) => inner.next(),
            Two::Right(inner) => inner.next(),
        }
    }

    fn fold<Acc, G: FnMut(Acc, Self::Item) -> Acc>(self, init: Acc, f: G) -> Acc {
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

impl<L: DoubleEndedIterator, R: DoubleEndedIterator<Item = L::Item>> DoubleEndedIterator for Two<L, R> {
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

    fn rfold<Acc, G: FnMut(Acc, Self::Item) -> Acc>(self, init: Acc, f: G) -> Acc {
        match self {
            Two::Left(inner) => inner.rfold(init, f),
            Two::Right(inner) => inner.rfold(init, f),
        }
    }
}

fn evens_or_odds(evens: bool) -> Two<impl DoubleEndedIterator<Item = u32>, impl DoubleEndedIterator<Item = u32>> {
    match evens {
        true => Two::Left((0..10).filter(|n| n % 2 == 0)),
        false => Two::Right((0..10).filter(|n| n % 2 == 1)),
    }
}

fn main() {
    println!("{:?}", evens_or_odds(true).collect::<Vec<_>>());
    println!("{:?}", evens_or_odds(false).rev().collect::<Vec<_>>());
    println!("{:?}", evens_or_odds(false).rev().nth(1));
    println!("{:?} {:?}", evens_or_odds(true).nth(2), evens_or_odds(true).nth_back(3));
    println!("{}", evens_or_odds(false).rev().fold(0, |sum, n| sum * 10 + n));
}
