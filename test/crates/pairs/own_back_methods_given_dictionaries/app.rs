//! A consumer that steps the library's `Two` of two filters from its back,
//! by its own `rfold` and `nth_back`, each given its sides' dictionaries,
//! as either's probe's `evens_or_odds(false).rev()` does (#475).
use dep::Two;

fn evens_or_odds(evens: bool) -> Two<impl DoubleEndedIterator<Item = u32>, impl DoubleEndedIterator<Item = u32>> {
    match evens {
        true => Two::Left((0..10).filter(|n| n % 2 == 0)),
        false => Two::Right((0..10).filter(|n| n % 2 == 1)),
    }
}

pub fn main() {
    println!("{:?}", evens_or_odds(true).collect::<Vec<_>>());
    println!("{:?}", evens_or_odds(false).rev().collect::<Vec<_>>());
    println!("{:?}", evens_or_odds(true).rev().map(|n| n * 10).collect::<Vec<_>>());
    println!("{:?}", evens_or_odds(false).rev().nth(1));
    println!("{:?}", evens_or_odds(true).nth_back(3));
    println!("{}", evens_or_odds(false).rev().fold(0, |sum, n| sum * 10 + n));
    let mut back = evens_or_odds(true).rev().skip(1);
    println!("{:?} {:?}", back.next(), back.last());
}
