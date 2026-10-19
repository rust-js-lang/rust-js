// What a `Some` or an `Ok` holds, an `Option` of what could look like
// `None`, given as it's held, box and all: `unwrap`, `unwrap_or`,
// `unwrap_or_else` and `unwrap_or_default`; `min_by`'s pick, boxed as
// `$minBy` makes it; and std's iterator of it, `into_iter()`'s (ADR 0472).
fn first_or<I: Iterator>(cur: Option<Option<I::Item>>, mut it: I) -> Option<I::Item> {
    cur.unwrap_or_else(|| it.next())
}

fn least<I: Iterator, F: FnMut(&I::Item, &I::Item) -> std::cmp::Ordering>(it: I, f: F) -> Vec<I::Item> {
    it.min_by(f).into_iter().collect()
}

fn most<T: Ord + Clone>(items: &[T]) -> Option<T> {
    items.iter().cloned().max_by_key(|x| x.clone())
}

fn unwraps<T: Clone>(a: Option<Option<T>>, r: Result<Option<T>, u8>) -> [Option<T>; 5] {
    [
        a.clone().unwrap(),
        a.clone().unwrap_or(None),
        a.clone().unwrap_or_default(),
        r.clone().unwrap_or_else(|_| None),
        r.unwrap_or_default(),
    ]
}

fn main() {
    let a: Option<Option<Option<u8>>> = Some(None);
    println!("{:?}", first_or(a, vec![Some(1u8)].into_iter()));
    let b: Option<Option<Option<u8>>> = None;
    println!("{:?}", first_or(b, vec![None, Some(2u8)].into_iter()));
    println!("{:?}", least(vec![Some(3u8), None, Some(1)].into_iter(), |x, y| x.cmp(y)));
    println!("{:?}", least(Vec::<Option<u8>>::new().into_iter(), |x, y| x.cmp(y)));
    println!("{:?}", most(&[None, Some(4u8), Some(2)]));
    println!("{:?}", most::<Option<u8>>(&[None]));
    println!("{:?}", unwraps(Some(Some(None::<u8>)), Ok(Some(None))));
    println!("{:?}", unwraps(Some(Some(Some(5u8))), Err(1)));
}
