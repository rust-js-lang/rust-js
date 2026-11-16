// A `copied()` of generic code's source, its length asked of a copy of it,
// a clone or one a generic function makes, and stepped from its back
// itself: made knowing its length and its back, each what's asked of a
// value of its type, never one for the other (#1983).
fn dup<I: Clone>(it: &I) -> I {
    it.clone()
}

fn cloned<'a, I>(src: I)
where
    I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone,
{
    let mut a = src.copied();
    println!("a next {:?}", a.next());
    let b = a.clone();
    println!("b len {}", b.len());
    println!("a next_back {:?}", a.next_back());
    println!("b rest {:?}", b.collect::<Vec<_>>());
    println!("a len {} rest {:?}", a.len(), a.collect::<Vec<_>>());
}

fn duped<'a, I>(src: I)
where
    I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone,
{
    let mut a = src.copied();
    println!("a next {:?}", a.next());
    let b = dup(&a);
    println!("b len {}", b.len());
    println!(
        "a next_back {:?} nth_back {:?}",
        a.next_back(),
        a.nth_back(1)
    );
    println!("a rfold {}", a.rfold(0, |sum, x| sum * 10 + x));
    println!("b rest {:?}", b.collect::<Vec<_>>());
}

fn main() {
    cloned([1u32, 2, 3, 4, 5].iter());
    duped([1u32, 2, 3, 4, 5].iter());
}
