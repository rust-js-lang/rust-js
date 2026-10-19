// `sum()` into a type parameter `S: Sum<A>`, `product()` into a `P:
// Product<A>`, and `collect()` into what a bound says, `Result<U, E>:
// FromIterator<..>`: each its caller's dictionary's, std's of a number a
// `reduce` by its operation (ADR 0476).
use std::iter::{Product, Sum, once};

fn sum1<I: Iterator, S: Sum<I::Item>>(mut it: I) -> Option<S> {
    it.next().map(|first| once(first).chain(it).sum())
}

fn product1<I: Iterator, P: Product<I::Item>>(mut it: I) -> Option<P> {
    it.next().map(|first| once(first).chain(it).product())
}

fn total<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().sum()
}

fn try_collect<I, T, U, E>(it: I) -> Result<U, E>
where
    I: Iterator<Item = Result<T, E>>,
    Result<U, E>: FromIterator<Result<T, E>>,
{
    it.collect()
}

fn main() {
    println!("{:?}", sum1::<_, i32>(vec![1, 2, 3].into_iter()));
    println!("{:?}", sum1::<_, f64>(Vec::<f64>::new().into_iter()));
    println!("{:?}", sum1::<_, f64>(vec![-0.0, -0.0].into_iter()));
    println!("{:?}", product1::<_, u64>(vec![2u64, 3, 4].into_iter()));
    println!("{:?}", product1::<_, u8>(vec![16u8, 16].into_iter().map(|x| x.wrapping_add(0))));
    println!("{}", total::<u32>(vec![200, 100]));
    let ok: Result<Vec<u8>, String> = try_collect(vec![Ok(1), Ok(2)].into_iter());
    let err: Result<Vec<u8>, String> = try_collect(vec![Ok(1), Err("bad".to_string()), Ok(3)].into_iter());
    println!("{ok:?} {err:?}");
}
