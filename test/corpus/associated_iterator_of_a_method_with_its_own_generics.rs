//@ library-differs
// A trait method with generics of its own, a type's, `items<U: Copy>`, or a
// constant's, `hidden<const M: u32>`, giving an iterator that knows its
// back: generic code steps it from its back, as the impl's dictionary makes
// it a JS iterator that knows it, where it threw `it.back is not a
// function` (#1308). `tagged<U: Copy>`'s closure copies its `U`, an `FnMut`
// only by the method's own bound. Of an impl of its own generics too. The
// opaque iterators are stepped where they're made: one a variable holds is
// refused in a library build (#1296).
trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items<U: Copy>(n: u32, tag: U) -> Self::Iter;
    fn hidden<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32>;
    fn tagged<U: Copy>(n: u32, tag: U) -> impl DoubleEndedIterator<Item = (u32, U)>;
}

struct Tens;

impl Source for Tens {
    type Iter = std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32>;
    fn items<U: Copy>(n: u32, _tag: U) -> Self::Iter {
        (0..n).map(|x| x * 10)
    }
    fn hidden<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x * M)
    }
    fn tagged<U: Copy>(n: u32, tag: U) -> impl DoubleEndedIterator<Item = (u32, U)> {
        (0..n).map(move |x| (x, tag))
    }
}

struct Scaled<const K: u32>;

impl<const K: u32> Source for Scaled<K> {
    type Iter = std::iter::Rev<std::ops::Range<u32>>;
    fn items<U: Copy>(n: u32, _tag: U) -> Self::Iter {
        (K..K + n).rev()
    }
    fn hidden<const M: u32>(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x * M + K)
    }
    fn tagged<U: Copy>(n: u32, tag: U) -> impl DoubleEndedIterator<Item = (u32, U)> {
        (K..K + n).map(move |x| (x, tag))
    }
}

fn from_its_back<S: Source>() -> (Option<u32>, Vec<u32>) {
    let mut it = S::items(4, 'a');
    (it.next_back(), it.rev().collect())
}

fn hidden<S: Source>() -> (Vec<u32>, u32) {
    let reversed = S::hidden::<3>(4).rev().collect();
    let folded = S::hidden::<2>(3).rfold(0, |sum, x| sum * 10 + x);
    (reversed, folded)
}

fn tagged<S: Source>() -> Vec<(u32, &'static str)> {
    S::tagged(3, "t").rev().collect()
}

fn main() {
    println!("{:?}", from_its_back::<Tens>());
    println!("{:?}", hidden::<Tens>());
    println!("{:?}", tagged::<Tens>());
    println!("{:?}", from_its_back::<Scaled<5>>());
    println!("{:?}", hidden::<Scaled<5>>());
    println!("{:?}", tagged::<Scaled<5>>());
}
