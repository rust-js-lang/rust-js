//@ library-differs
// A trait's associated `IntoIterator` whose `IntoIter` it bounds
// `DoubleEndedIterator`: generic code's `flat_map` of what it gives,
// stepped from its back, makes each inner iterator knowing its back by
// the trait's dictionary, `PairsIntoIterDoubleEndedIterator`, the bound
// it declares on the type's `IntoIter`, where the plain build threw a
// `TypeError` as it stepped each from a back it hadn't, and the build of
// std's structs refused it (#1355). Of a `Vec` and of an array.
trait Source {
    type Pairs: IntoIterator<Item = u32, IntoIter: DoubleEndedIterator>;
    fn pairs(n: u32) -> Self::Pairs;
}

struct Vecs;

impl Source for Vecs {
    type Pairs = Vec<u32>;
    fn pairs(n: u32) -> Vec<u32> {
        vec![n; n as usize]
    }
}

struct Arrays;

impl Source for Arrays {
    type Pairs = [u32; 2];
    fn pairs(n: u32) -> [u32; 2] {
        [n, n * 10]
    }
}

fn folded<S: Source>() -> u32 {
    (1..4u32).flat_map(S::pairs).rfold(0, |a, x| a * 10 % 1000 + x)
}

fn last<S: Source>() -> Option<u32> {
    (1..4u32).flat_map(S::pairs).next_back()
}

fn reversed<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(S::pairs).rev().collect()
}

fn main() {
    println!("{} {:?} {:?}", folded::<Vecs>(), last::<Vecs>(), reversed::<Vecs>());
    println!(
        "{} {:?} {:?}",
        folded::<Arrays>(),
        last::<Arrays>(),
        reversed::<Arrays>()
    );
}
