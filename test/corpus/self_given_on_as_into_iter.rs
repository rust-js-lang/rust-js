// A default's `&mut Self`, given as it is, given on through a dictionary
// where an `IntoIterator` goes, as itertools' `next_tuple` does.

trait Collect: Sized {
    fn from_iter_no_buf<I: IntoIterator<Item = u32>>(iter: I) -> Option<Self>;
}

impl Collect for (u32, u32) {
    fn from_iter_no_buf<I: IntoIterator<Item = u32>>(iter: I) -> Option<Self> {
        let mut it = iter.into_iter();
        Some((it.next()?, it.next()?))
    }
}

trait Ext: Iterator<Item = u32> {
    fn next_pair<T: Collect>(&mut self) -> Option<T> {
        T::from_iter_no_buf(self)
    }
}

impl<I: Iterator<Item = u32>> Ext for I {}

fn pair_of<I: Iterator<Item = u32>>(it: &mut I) -> Option<(u32, u32)> {
    <(u32, u32)>::from_iter_no_buf(it)
}

fn main() {
    let mut r = 0..5u32;
    let p: Option<(u32, u32)> = r.next_pair();
    println!("{p:?} {:?}", r.next());
    let mut v = vec![7, 8, 9].into_iter();
    println!("{:?} {:?}", pair_of(&mut v), v.next());
    let mut q = 10..15u32;
    println!("{:?} {:?}", pair_of(&mut q), q.next());
}
