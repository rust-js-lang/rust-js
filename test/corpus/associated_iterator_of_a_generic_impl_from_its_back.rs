//@ library-differs
// #1292's shape, an associated iterator std's struct of an adapter, and an
// `impl DoubleEndedIterator` a method hides it behind, of a generic impl and
// of a const-generic one: what the method gives names the impl's
// parameters, which the dictionary's environment holds, so it's normalized
// to std's struct, stepped from its back by its compiled `next_back` (#1292,
// #1304). A flatten of them is the build of std's structs' refusal, as
// #1292's own case's is.
use std::marker::PhantomData;

trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(n: u32) -> Self::Iter;
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32>;
}

struct Tens<T>(PhantomData<T>);

impl<T: Copy> Source for Tens<T> {
    type Iter = std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>;
    fn items(n: u32) -> Self::Iter {
        (0..n).map(|x| x * 10)
    }
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x + 1)
    }
}

struct Scaled<const K: u32>;

impl<const K: u32> Source for Scaled<K> {
    type Iter = std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>;
    fn items(n: u32) -> Self::Iter {
        (0..n).map(|x| x * 3)
    }
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x * K)
    }
}

fn from_its_back<S: Source>() -> (Option<u32>, Vec<u32>) {
    let mut it = S::items(4);
    (it.next_back(), it.rev().collect())
}

fn hidden<S: Source>() -> Vec<u32> {
    S::hidden(3).rev().collect()
}

fn main() {
    println!("{:?}", from_its_back::<Tens<u8>>());
    println!("{:?}", hidden::<Tens<u8>>());
    println!("{:?}", from_its_back::<Scaled<5>>());
    println!("{:?}", hidden::<Scaled<5>>());
}
