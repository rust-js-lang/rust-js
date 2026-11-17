// A clone of a generic `copied()`, stepped from its back, while the
// original is collected from its front: each steps its own items (#1984).
fn cloned<'a, I>(src: I)
where
    I: Iterator<Item = &'a u32> + DoubleEndedIterator + ExactSizeIterator + Clone,
{
    let a = src.copied();
    let mut b = a.clone();
    println!("{:?} {:?}", b.next_back(), a.collect::<Vec<_>>());
    println!("{:?} {}", b.next_back(), b.len());
}

fn main() {
    cloned([1u32, 2, 3].iter());
    cloned([7u32].iter());
    cloned(Vec::<u32>::new().iter());
}
