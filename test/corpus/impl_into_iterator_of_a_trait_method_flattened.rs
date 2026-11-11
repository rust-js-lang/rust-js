//@ library-differs
// A trait method's `impl IntoIterator` alone, which no `Iterator` bound
// gives an `IntoIterator` of: generic code's `flat_map` of what it gives
// finds the impl's `IntoIterator` in the trait's dictionary, as an
// associated type's, `PairsIntoIterator`, named by its method, where it
// was refused for want of that evidence (#1345). Of an array and of a
// `Vec`, collected and folded from the back.
trait Source {
    fn pairs(n: u32) -> impl IntoIterator<Item = u32, IntoIter: DoubleEndedIterator>;
}

struct Arrays;

impl Source for Arrays {
    fn pairs(n: u32) -> impl IntoIterator<Item = u32, IntoIter: DoubleEndedIterator> {
        [n, n * 10]
    }
}

struct Vecs;

impl Source for Vecs {
    fn pairs(n: u32) -> impl IntoIterator<Item = u32, IntoIter: DoubleEndedIterator> {
        vec![n; n as usize]
    }
}

fn paired<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(S::pairs).collect()
}

fn folded<S: Source>() -> u32 {
    (1..4u32).flat_map(S::pairs).rfold(0, |a, x| a * 10 % 1000 + x)
}

fn main() {
    println!("{:?} {}", paired::<Arrays>(), folded::<Arrays>());
    println!("{:?} {}", paired::<Vecs>(), folded::<Vecs>());
}
