//@ library-differs
// Generic code's flatten of what a trait's associated types and a trait
// method's `impl Trait` give, collected where the crate's own `size_hint`
// prints, and stepped from its back: std's struct, held, asks the constant
// size of their items, which the trait's dictionaries carry,
// `ItemsConstSize` an array's, `IterConstSize` and `HiddenConstSize` none.
// Made in place it asked the hint again as the `Vec` grew where a caller
// made them an array (#1341); held in the build of std's structs, it was
// refused, no dictionary carrying a trait method's `impl Trait`'s (#1312).
struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
}

trait Source {
    type Items: IntoIterator<Item = u32>;
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(n: u32) -> Self::Items;
    fn iter(n: u32) -> Self::Iter;
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32>;
}

struct Pairs;

impl Source for Pairs {
    type Items = [u32; 2];
    type Iter = std::ops::Range<u32>;
    fn items(n: u32) -> [u32; 2] {
        [n, n + 10]
    }
    fn iter(n: u32) -> std::ops::Range<u32> {
        0..n
    }
    fn hidden(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x * 10)
    }
}

fn gathered<S: Source>() -> Vec<u32> {
    Own(4).flat_map(|n| S::items(n)).collect()
}

fn back<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(|n| S::iter(n)).rev().collect()
}

fn hidden_back<S: Source>() -> Vec<u32> {
    (1..4u32).flat_map(S::hidden).rev().collect()
}

fn main() {
    println!("{:?}", gathered::<Pairs>());
    println!("{:?}", back::<Pairs>());
    println!("{:?}", hidden_back::<Pairs>());
}
