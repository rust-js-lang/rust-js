// A held iterator whose adapters hold one made in place, a `filter` of a
// `skip(1)`, made in one function and its hint asked in another: what
// leaves the function that makes it is made knowing its hint there, so
// `size_hint()` is std's wherever it's asked (#1427).

type Skipped = std::iter::Filter<std::iter::Skip<std::vec::IntoIter<u8>>, fn(&u8) -> bool>;

fn made() -> Skipped {
    vec![1u8, 2, 3].into_iter().skip(1).filter(|x| *x > 1)
}

fn hint(it: &Skipped) -> (usize, Option<usize>) {
    it.size_hint()
}

fn main() {
    let mut it = made();
    println!("{:?}", hint(&it));
    it.next();
    println!("{:?} {:?}", hint(&it), it.size_hint());
}
