// An `enumerate()` made where it's given to generic code that steps its
// back, then counted there: `$enumerate`'s `count` is how many it's given,
// as std's `Enumerate`'s is, not its `count()` (ADR 0988).
fn counted<I: DoubleEndedIterator>(mut it: I) -> (bool, usize) {
    let had = it.next_back().is_some();
    (had, it.count())
}

fn main() {
    let v = vec![10u32, 20, 30];
    println!("{:?}", counted(v.iter().enumerate()));
    println!("{:?}", counted(v.iter().copied().enumerate().skip(1)));
    let empty: Vec<u32> = Vec::new();
    println!("{:?}", counted(empty.iter().enumerate()));
}
