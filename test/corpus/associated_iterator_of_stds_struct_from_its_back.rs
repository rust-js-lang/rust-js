//@ library-differs
// A trait's associated iterator, and what its method hides behind an
// `impl`, that an impl gives as std's struct of an adapter (#1226):
// generic code has it as the JS iterator the impl's dictionary makes of
// it, knowing its back by std's compiled `next_back` where the trait
// bounds it so, as it has the crate's own (#978). It was taken for one
// that knew its back already and stepped as an empty one (#1292). A
// flatten of them is test/lowering-iterators.test.ts's, which the build of
// std's structs alone refuses.

trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(n: u32) -> Self::Iter;
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32>;
}

struct Tens;

impl Source for Tens {
    type Iter = std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>;
    fn items(n: u32) -> Self::Iter {
        (0..n).map(|x| x * 10)
    }
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x + 1)
    }
}

fn from_its_back<S: Source>() -> (Option<u32>, Vec<u32>) {
    let mut it = S::items(4);
    (it.next_back(), it.rev().collect())
}

fn hidden<S: Source>() -> Vec<u32> {
    S::hidden(3).rev().collect()
}

fn folded<S: Source>() -> u32 {
    S::items(4).rfold(0, |a, x| a * 2 + x)
}

fn main() {
    println!("{:?}", from_its_back::<Tens>());
    println!("{:?}", hidden::<Tens>());
    println!("{:?}", folded::<Tens>());
    println!("{:?}", Tens::items(3).rev().collect::<Vec<_>>());
}
