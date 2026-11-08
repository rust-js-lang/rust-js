//@ compile-fail: whose facts its caller's dictionary doesn't carry (#934)
// A generic caller bounded by the crate's trait that extends `Iterator`,
// `T: Sorted`, can't give `sorted` what std's stable sort picks by
// `T::Item` (#925): `Sorted`'s dictionary doesn't carry facts of
// `Iterator`'s associated type (#934). `run-pass` again with it, its sort
// std's, as `extension_trait_params` is without one.
trait Sorted: Iterator {
    fn sorted(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.sort();
        v
    }
}

impl<I: Iterator> Sorted for I {}

fn through<T: Sorted>(t: T) -> Vec<T::Item>
where
    T::Item: Ord,
{
    t.sorted()
}

fn main() {
    println!("{:?}", through(vec![5, 3, 9, 1].into_iter()));
    println!("{:?}", vec![2, 1].into_iter().sorted());
}
