//! A library's `FromIterator` impl that asks the hint of what it's given,
//! which its own generic code collects into, so its JS exports the impl's
//! dictionary, `hintedFromIteratorU32` (#2515).
pub struct Hinted(pub usize, pub Option<usize>);

impl FromIterator<u32> for Hinted {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.into_iter().size_hint();
        Hinted(low, high)
    }
}

fn gather<B: FromIterator<u32>>(v: Vec<u32>) -> B {
    v.into_iter().collect()
}

pub fn own_hint(v: Vec<u32>) -> usize {
    let Hinted(low, _) = gather(v);
    low
}
