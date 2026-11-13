//@ library-differs
// A trait's default's `flat_map` of its items, `Self::Item: IntoIterator`
// by a where clause, collected into a `Vec`: where its caller's items are
// arrays and its source is `TrustedLen`, std's `FlatMap<I, [T; N], F>` is
// `TrustedLen` too, so `collect` asks room for exactly its hint's upper
// bound, 3 of one array of three, not `max(4, lower + 1)` after its first
// item as of one that isn't, a `Vec` of `Vec`s' (#1614). Whether it is,
// its callers' to say, as only they know what its items are. So too of
// generic code's `chain` of its type parameter's iterator and a `Vec`'s,
// `TrustedLen` where the parameter is: room for exactly 5 of four and one.
pub trait Ext: Iterator {
    fn flat(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).collect()
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

fn chained<'a, I: Iterator<Item = &'a str>>(it: I) -> Vec<&'a str> {
    it.chain(vec!["x"]).collect()
}

fn main() {
    let one: Vec<u32> = vec![[1u32, 2, 3]].into_iter().flat();
    println!("{} {}", one.len(), one.capacity());
    let two: Vec<u32> = vec![[1u32, 2], [3, 4]].into_iter().flat();
    println!("{} {}", two.len(), two.capacity());
    let refs: Vec<&u32> = [[5u32, 6, 7]].iter().flat();
    println!("{} {}", refs.len(), refs.capacity());
    let vecs: Vec<u32> = vec![vec![1u32, 2, 3]].into_iter().flat();
    println!("{} {}", vecs.len(), vecs.capacity());
    let filtered: Vec<u32> = vec![[1u32, 2, 3]].into_iter().filter(|_| true).flat();
    println!("{} {}", filtered.len(), filtered.capacity());
    let trusted = chained(vec!["a", "b", "c", "d"].into_iter());
    println!("{} {}", trusted.len(), trusted.capacity());
    let split = chained("a b c d".split(' '));
    println!("{} {}", split.len(), split.capacity());
}
