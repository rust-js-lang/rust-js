//@ library-differs
// A zip of a type parameter's sides, a generic function's and a trait's
// default's over `Self`: std's `Zip` specializes on whether each side is
// `TrustedRandomAccess` or `TrustedLen`, which only the callers know, so
// it stays the recognizers' lazy zip, which asks neither (ADR 1136).
pub trait Tools: Iterator {
    fn dot(self, other: Vec<i32>) -> i32
    where
        Self: Iterator<Item = i32> + Sized,
    {
        self.zip(other).map(|(a, b)| a * b).sum()
    }
}

impl<T: Iterator + ?Sized> Tools for T {}

fn dot<I: Iterator<Item = i32>>(it: I, other: Vec<i32>) -> i32 {
    it.zip(other).map(|(a, b)| a * b).sum()
}

fn main() {
    println!("{}", vec![1, 2, 3].into_iter().dot(vec![4, 5, 6]));
    println!("{}", dot([1, 2].into_iter(), vec![3, 4]));
}
