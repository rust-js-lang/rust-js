// A filter's shapes rust-js refused before the filter family's flip, each
// std's compiled `Filter` now and run as native Rust runs it (#1681's
// change 2): a held `rev()` of a filter of a `skip(1)` its hint asked
// (#1427), a `peekable()` of a filter made here its hint asked (#382),
// and a filter generic code makes where its source isn't bound by
// `Clone`, cloned (ADR 0525).
pub fn rev_of_filter_of_skip() -> (usize, Option<usize>) {
    let it = vec![1u8, 2, 3].into_iter().skip(1).filter(|x| *x > 1).rev();
    it.size_hint()
}

pub fn peekable_filter(v: &[u32]) -> (usize, Option<usize>) {
    v.iter().filter(|x| **x > 1).peekable().size_hint()
}

pub fn evens<I: Iterator<Item = i32>>(it: I) -> std::iter::Filter<I, fn(&i32) -> bool> {
    it.filter(|x| x % 2 == 0)
}

pub fn cloned_filter() -> usize {
    let d = evens(1..9);
    d.clone().count()
}

fn main() {
    println!("{:?}", rev_of_filter_of_skip());
    println!("{:?}", peekable_filter(&[1, 2, 3]));
    println!("{:?}", peekable_filter(&[]));
    println!("{}", cloned_filter());
}
