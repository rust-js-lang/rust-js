// A `take()` and an `enumerate()` made in place and given to generic code
// that asks their back and length, stepped from the front and the back,
// counted and cloned part-way, as std's `Take` and `Enumerate` step them
// (ADR 0542; #382).
fn ends<I: DoubleEndedIterator + ExactSizeIterator + Clone>(
    mut it: I,
) -> (usize, Option<I::Item>, Option<I::Item>, usize, Vec<I::Item>) {
    let first = it.next();
    let len = it.len();
    let copy = it.clone();
    let last = it.next_back();
    let rest: Vec<I::Item> = copy.collect();
    (len, first, last, it.len(), rest)
}

fn main() {
    let v = vec![10, 20, 30, 40, 50];
    println!("{:?}", ends(v.iter().take(3)));
    println!("{:?}", ends(v.iter().enumerate()));
    println!("{:?}", ends(v.iter().copied().take(4).enumerate()));
    let mut kept = v.iter().take(4).filter(|x| **x != 20);
    println!(
        "{:?} {:?} {:?} {:?}",
        kept.next(),
        kept.next_back(),
        kept.next(),
        kept.next()
    );
    let mut counted = v.iter().enumerate().filter(|(i, _)| *i != 1);
    let again = counted.clone();
    println!("{:?} {:?} {:?}", counted.next(), counted.next_back(), counted.next());
    println!("{:?}", again.collect::<Vec<_>>());
    let mut tail = v.iter().skip(1).take(3).filter(|_| true);
    println!("{:?} {:?} {:?}", tail.next_back(), tail.next(), tail.next());
}
