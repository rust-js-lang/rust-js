// A flatten of iterators stepped from its back (#260): each inner iterator
// made knowing its back as it's taken, an `Option`'s slice's iterator, a
// range, std's `Enumerate` and `Rev` held in an `Option`, a `Vec` of
// iterators and what `flat_map` gives, from either end, as std's `Flatten`
// steps them.
fn main() {
    let held = Some([1, 2, 3].iter());
    println!("{}", held.into_iter().flatten().rfold(0, |a, x| a * 10 + x));
    let range = Some(1..4);
    println!("{:?}", range.into_iter().flatten().next_back());

    let numbered = Some([7, 8, 9].iter().enumerate());
    println!("{}", numbered.into_iter().flatten().rfold(0, |a, (i, x)| a * 10 + i + x));
    let reversed = Some((1..4).rev());
    println!("{:?}", reversed.into_iter().flatten().next_back());

    let groups = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let inner: Vec<_> = groups.iter().map(|g| g.iter()).collect();
    let mut both = inner.into_iter().flatten();
    println!("{:?} {:?} {:?}", both.next_back(), both.next(), both.next_back());
    println!("{:?}", both.collect::<Vec<_>>());

    let back: Vec<_> = groups.iter().flat_map(|g| g.iter().map(|x| x * 10)).rev().collect();
    println!("{back:?}");
}
