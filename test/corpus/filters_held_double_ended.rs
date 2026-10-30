//@ library-differs
// Filters and maps an enum holds, stepped from their back by its
// `DoubleEndedIterator`, which asks it of what it holds, as `either`'s
// `Either` does: made knowing their back where they're made.

enum Two<L, R> {
    One(L),
    Other(R),
}

impl<L: Iterator, R: Iterator<Item = L::Item>> Iterator for Two<L, R> {
    type Item = L::Item;

    fn next(&mut self) -> Option<L::Item> {
        match self {
            Two::One(l) => l.next(),
            Two::Other(r) => r.next(),
        }
    }
}

impl<L: DoubleEndedIterator, R: DoubleEndedIterator<Item = L::Item>> DoubleEndedIterator for Two<L, R> {
    fn next_back(&mut self) -> Option<L::Item> {
        match self {
            Two::One(l) => l.next_back(),
            Two::Other(r) => r.next_back(),
        }
    }
}

fn evens_or_tens(evens: bool) -> Two<impl DoubleEndedIterator<Item = u32>, impl DoubleEndedIterator<Item = u32>> {
    match evens {
        true => Two::One((0..10).filter(|n| n % 2 == 0)),
        false => Two::Other((1..4).map(|n| n * 10)),
    }
}

fn main() {
    let evens: Vec<u32> = evens_or_tens(true).rev().collect();
    let tens: Vec<u32> = evens_or_tens(false).rev().collect();
    let mut both = evens_or_tens(true);
    println!("{evens:?} {tens:?} {:?} {:?}", both.next(), both.next_back());
}
