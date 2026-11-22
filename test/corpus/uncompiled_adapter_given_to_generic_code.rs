// A type parameter, or an `IntoIter`, held only in std's adapter, given
// to generic code or given back: `Copied<I>`, `Filter<I, P>`, `Rev<I>`,
// `Zip<I, J>` and `Chain<A, B>`, each a JS iterator whose caller steps it
// and never reads what it holds, so what it holds isn't held as it is
// (`holds_readably`, #2331); each stepped from both ends where it can be,
// as Rust steps it.
use std::iter::{Chain, Copied, Filter, Rev, Zip};

#[derive(Debug, Clone)]
struct Down(u32);

impl Iterator for Down {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Down {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|x| x + 100)
    }
}

fn ends<'a, I>(mut it: Copied<I>) -> (Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = &'a u32>,
{
    (it.next_back(), it.next())
}

fn kept<I, P>(it: Filter<I, P>) -> Vec<u32>
where
    I: Iterator<Item = u32>,
    P: FnMut(&u32) -> bool,
{
    it.collect()
}

fn back<I: DoubleEndedIterator<Item = u32>>(mut it: Rev<I>) -> Option<u32> {
    it.next();
    it.next()
}

fn pairs<I, J>(it: Zip<I, J>) -> Vec<(u32, char)>
where
    I: Iterator<Item = u32>,
    J: Iterator<Item = char>,
{
    it.collect()
}

fn joined<A, B>(it: Chain<A, B>) -> u32
where
    A: Iterator<Item = u32>,
    B: Iterator<Item = u32>,
{
    it.sum()
}

fn zipped<T: IntoIterator, U: IntoIterator>(t: T, u: U) -> Zip<T::IntoIter, U::IntoIter> {
    t.into_iter().zip(u)
}

fn chained<T: IntoIterator<Item = u32>>(t: T, more: u32) -> Chain<T::IntoIter, std::ops::Range<u32>> {
    t.into_iter().chain(0..more)
}

fn main() {
    let v = [1u32, 2, 3];
    println!("{:?}", ends(v.iter().copied()));
    println!("{:?}", ends([7u32].iter().copied()));
    println!("{:?}", kept((0..10).filter(|x| x % 3 == 0)));
    println!("{:?}", kept(Down(5).filter(|x| *x != 2)));
    println!("{:?}", back((1..5).rev()));
    println!("{:?}", back(Down(4).rev()));
    println!("{:?}", pairs((1..4).zip("abc".chars())));
    println!("{:?}", pairs(Down(2).zip("xyz".chars())));
    println!("{}", joined((1..3).chain(Down(3))));
    let z: Vec<_> = zipped(3..6, Down(5)).collect();
    println!("{z:?}");
    let z: Vec<_> = zipped(['a', 'b'], [true, false, true]).collect();
    println!("{z:?}");
    let mut c = chained(Down(2), 2);
    println!("{:?} {:?}", c.next(), c.next_back());
    println!("{:?}", c.collect::<Vec<_>>());
}
