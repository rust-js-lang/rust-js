// A std iterator stepped by hand, `it.next()`, is a JS iterator its
// helpers step in place: what's left is what's taken after (ADR 0364).
// A generic one's too, whose `&mut` is the iterator.
fn first_two<I: Iterator<Item = char>>(mut it: I) -> (Option<char>, Option<char>, usize) {
    let a = it.next();
    let b = it.next();
    (a, b, it.count())
}

fn main() {
    let mut it = "abcd".chars();
    let first = it.next();
    let second = it.next();
    let rest: String = it.collect();
    println!("{:?} {:?} {}", first, second, rest);
    let mut range = 1..5;
    range.next();
    let total: u32 = range.sum();
    println!("{total}");
    println!("{:?}", first_two("xyz".chars()));
}
