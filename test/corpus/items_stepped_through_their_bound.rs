// An iterator's items that are iterators themselves, stepped by generic
// code through a bound of its items, `S::Item: Iterator`, a projection no
// type parameter names: they come as their caller holds them, the crate's
// iterator of a field its object, a range its own, std's iterators theirs,
// so they're stepped by that bound's dictionary, as an `S::Item:
// IntoIterator`'s are (#278), never as JS iterators (#2541). One JS holds as
// no object is refused (`placed_items_stepped_through_their_bound`).

#[derive(Debug, Clone)]
struct Counter {
    n: u8,
}

impl Iterator for Counter {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.n += 1;
        Some(self.n)
    }
}

fn heads<S>(s: S) -> Vec<Option<<S::Item as Iterator>::Item>>
where
    S: Iterator,
    S::Item: Iterator,
{
    s.map(|mut i| i.next()).collect()
}

fn flat<S>(s: S) -> Vec<<S::Item as Iterator>::Item>
where
    S: Iterator,
    S::Item: Iterator,
{
    s.flat_map(|i| i.take(2)).collect()
}

fn main() {
    println!("{:?}", heads([Counter { n: 0 }, Counter { n: 5 }].into_iter()));
    println!("{:?}", heads(vec![0..2u8, 5..9].into_iter()));
    println!("{:?}", heads(vec![vec![1u8].into_iter(), vec![].into_iter()].into_iter()));
    println!("{:?}", heads(["ab", "c"].iter().map(|s| s.chars())));
    println!("{:?}", flat([Counter { n: 1 }, Counter { n: 9 }].into_iter()));
    println!("{:?}", flat(vec![0..5u8, 7..8].into_iter()));
}
