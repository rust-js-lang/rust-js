// A `rev()` made where it's given to generic code that steps it from both
// ends or asks its length, as Rust steps it: of a range, an array's items,
// a map, a filter and a skip of them, and a `rev()` of a `rev()` (ADR 0987).
fn ends<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> (Option<u32>, Option<u32>, Option<u32>) {
    (it.next_back(), it.next(), it.next())
}

fn first<I: ExactSizeIterator<Item = u32>>(mut it: I) -> (usize, Option<u32>, usize) {
    let len = it.len();
    let first = it.next();
    (len, first, it.len())
}

fn main() {
    let v = vec![1u32, 2, 3, 4];
    println!("{:?} {:?}", ends((1..4u32).rev()), ends((1..=4u32).rev()));
    println!("{:?}", ends((0..3u32).rev().rev()));
    println!(
        "{:?} {:?}",
        ends(v.iter().copied().rev()),
        ends(v.iter().map(|x| x * 2).rev())
    );
    println!("{:?}", ends(v.iter().copied().filter(|x| x % 2 == 1).rev()));
    println!("{:?}", ends((1..9u32).skip(2).rev()));
    println!(
        "{:?} {:?}",
        first((1..4u32).rev()),
        first(v.iter().map(|x| x * 10).rev())
    );
}
