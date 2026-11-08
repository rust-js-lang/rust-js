//@ library-differs
// A generic function's `impl DoubleEndedIterator`, held in a struct, a
// filter and a map of closures of its type parameter's, is made knowing its
// back where it's made, as the non-generic one is: the caller steps it from
// its back through `Two`'s `next_back`, which asks it of what it holds; the
// set of what's stepped so has it in the function's own terms, `B`'s, not
// the caller's `bool`'s (#993).
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

fn evens_or_tens<B: Into<bool>>(evens: B) -> Two<impl DoubleEndedIterator<Item = u32>, impl DoubleEndedIterator<Item = u32>> {
    match evens.into() {
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
