// A trait's default that calls its trait on an adapter of `Self`,
// `self.enumerate().minmax()` as itertools' `position_minmax` does, of a
// blanket impl over `?Sized` types: its `Self: Sized` is the impl's, which
// the adapter asks (ADR 0541).
trait Ext: Iterator {
    fn widest(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut best = None;
        for item in self {
            if best.as_ref().is_none_or(|b| &item > b) {
                best = Some(item);
            }
        }
        best
    }

    fn position_max(self) -> Option<usize>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        self.enumerate().map(|(i, x)| (x, i)).widest().map(|(_, i)| i)
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

fn main() {
    println!("{:?}", vec![3, 9, 4].into_iter().position_max());
    println!("{:?}", "bca".chars().position_max());
    println!("{:?}", std::iter::empty::<u8>().position_max());
}
