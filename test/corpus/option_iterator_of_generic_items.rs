// std's iterator of a generic body's items, `option::IntoIter<I::Item>`, an
// `Option`'s `into_iter()` of what `I` gives: a JS iterator where it's
// stepped, `stepped.next()`, then counted, as std's is; chained, and
// counted whole (ADR 0472).

fn firsts<I: Iterator>(mut it: I) -> Vec<I::Item> {
    it.next().into_iter().chain(it.next()).collect()
}

fn first_twice<I: Iterator>(mut it: I) -> usize
where
    I::Item: Clone,
{
    let first = it.next();
    let mut stepped = first.clone().into_iter();
    let one = stepped.next().is_some();
    let rest = stepped.count();
    usize::from(one) + rest + first.into_iter().count()
}

fn main() {
    println!("{:?}", firsts([1, 2, 3].into_iter()));
    println!("{:?}", firsts("ab".chars()));
    println!("{:?}", firsts(std::iter::empty::<u8>()));
    println!("{}", first_twice([4, 5].into_iter()));
    println!("{}", first_twice(std::iter::empty::<u8>()));
}
