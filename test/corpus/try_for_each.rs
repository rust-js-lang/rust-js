// `try_for_each(f)`, std's `try_fold((), |(), x| f(x))`: each item's call,
// stopping at the first `Err` or `None`, which it gives, what's left still
// there to step; into a `fmt::Result`, each item's write; and `try_fold`
// into an `Option` of what could look like `None` (ADR 0539).
fn check(items: &[i32]) -> Result<(), String> {
    items.iter().try_for_each(|&x| if x < 0 { Err(format!("{x} is negative")) } else { Ok(()) })
}

fn positive_from<I: Iterator<Item = i32>>(it: &mut I) -> Option<()> {
    it.try_for_each(|x| (x > 0).then_some(()))
}

// Into an `Option` of what could look like `None`: `Some(None)` apart from
// `None`.
fn latest(items: &[Option<u32>]) -> Option<Option<u32>> {
    items.iter().try_fold(None, |seen, &x| if x == Some(0) { None } else { Some(x.or(seen)) })
}

fn main() {
    println!("{:?} {:?}", check(&[1, 2]), check(&[1, -2, -3]));
    let mut it = vec![3, 0, 5].into_iter();
    println!("{:?} {:?}", positive_from(&mut it), it.next());
    let mut all = 1..3;
    println!("{:?} {:?}", positive_from(&mut all), all.next());
    println!("{:?} {:?} {:?}", latest(&[Some(1), None]), latest(&[None]), latest(&[Some(2), Some(0)]));
}
