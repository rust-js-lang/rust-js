//@ library-differs
// A generic caller bounded by the crate's trait that extends `Iterator`,
// `T: Sorted`, gives `sorted` what std's stable sort picks by `T::Item`
// (#925): `<T as Iterator>::Item` is one of its slots, as it would be of
// `T: Sorted + Iterator`, its facts its caller's to give (#934). So an
// order the program writes, which counts its comparisons, is asked as std
// asks it.
use std::cell::Cell;
use std::cmp::Ordering;

thread_local! {
    static COMPARED: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug, PartialEq, Eq)]
struct Counted(u32, char);

impl PartialOrd for Counted {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Counted {
    fn cmp(&self, other: &Self) -> Ordering {
        COMPARED.set(COMPARED.get() + 1);
        self.0.cmp(&other.0)
    }
}

trait Sorted: Iterator {
    fn sorted(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.sort();
        v
    }
}

impl<I: Iterator> Sorted for I {}

fn through<T: Sorted>(t: T) -> Vec<T::Item>
where
    T::Item: Ord,
{
    t.sorted()
}

fn main() {
    println!("{:?}", through(vec![5, 3, 9, 1].into_iter()));
    println!("{:?}", vec![2, 1].into_iter().sorted());
    let items = [(3, 'a'), (1, 'b'), (3, 'c'), (2, 'd'), (1, 'e'), (9, 'f'), (0, 'g')];
    let sorted = through(items.into_iter().map(|(n, c)| Counted(n, c)));
    println!("{sorted:?} after {} comparisons", COMPARED.get());
}
