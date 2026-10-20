// An iterator given as it is, a type parameter another bound reads or a
// type holds, is given its extensions' dictionaries where it's stepped,
// `DoubleEndedIterator`'s `{ next_back }` and `ExactSizeIterator`'s
// `{ len }`, as it's given `Iterator`'s: `rev()` of it is stepped by its
// `next_back`; given on as a JS iterator, it's one made of them,
// `$iterator(it, IIterator.next, true, IDoubleEndedIterator.next_back)`;
// and one given to an adapter, `chain(it)`'s, is stepped by its dictionary
// (ADR 0487).
#[derive(Clone, Debug)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl ExactSizeIterator for Countdown {}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

fn twice<I>(iter: I) -> (usize, Vec<I::Item>)
where
    I: Iterator + Clone + ExactSizeIterator,
{
    let len = iter.len();
    (len, iter.clone().chain(iter).collect())
}

fn ends<I>(mut iter: I) -> (Option<I::Item>, Option<I::Item>, usize)
where
    I: Iterator + Clone + DoubleEndedIterator,
{
    let count = iter.clone().count();
    (iter.next_back(), iter.next(), count)
}

fn last<K: DoubleEndedIterator>(mut k: K) -> Option<K::Item> {
    k.next_back()
}

fn size<K: ExactSizeIterator>(k: K) -> usize {
    k.len()
}

fn passed_on<I>(iter: I) -> (Option<I::Item>, usize, Vec<I::Item>)
where
    I: Iterator + Clone + DoubleEndedIterator + ExactSizeIterator,
{
    (last(iter.clone()), size(iter.clone()), iter.rev().collect())
}

fn first<K: DoubleEndedIterator>(mut k: K) -> Option<K::Item> {
    k.next()
}

fn handed_on<I>(iter: I) -> Option<I::Item>
where
    I: Iterator + Clone + DoubleEndedIterator,
{
    first(iter)
}

fn backwards<I>(iter: I) -> Vec<I::Item>
where
    I: Iterator + Clone + DoubleEndedIterator,
{
    iter.rev().collect()
}

fn after<I>(iter: I) -> Vec<u32>
where
    I: Iterator<Item = u32> + Clone,
{
    (10..12).chain(iter).collect()
}

fn main() {
    println!("{:?}", twice(vec![1, 2, 3].into_iter()));
    println!("{:?}", twice(Countdown(2)));
    println!("{:?}", twice(0..3));
    let mut it = [4, 5, 6].iter();
    it.next();
    println!("{:?}", twice(it));

    println!("{:?}", ends(vec!['a', 'b', 'c'].into_iter()));
    println!("{:?}", ends(Countdown(3)));
    println!("{:?}", ends(10..12));

    println!("{:?}", passed_on(vec![1, 2, 3].into_iter()));
    println!("{:?}", passed_on(Countdown(2)));
    println!("{:?}", passed_on(0..3));

    println!("{:?} {:?}", handed_on(Countdown(2)), handed_on(5..7));
    println!("{:?} {:?}", backwards(Countdown(2)), backwards(5..7));
    println!("{:?} {:?}", after(Countdown(2)), after(5..7));
}
